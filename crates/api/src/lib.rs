//! # autohttps-api —— 共享 HTTP 表面(传输契约层,ARCHITECTURE §2 / AR1)
//!
//! 两形态(server/desktop)挂载**同一** axum Router:REST(camelCase JSON)+ 全局 SSE + 内嵌 SPA。
//! REST/SSE 统一挂在 `/api` 基路径下(避免与 SPA client-side 路由如 `/certificates` 冲突);
//! 前端 api 客户端以 `/api` 为 baseURL。资源命名遵 common §7。
//!
//! 进度:读取/CRUD、签发/续签/吊销执行、自签 CA、扫描器、SSE 实时推送、ACME 全线(账户/挑战/取证)均已实现。

pub mod dto;
pub mod embed;
pub mod error;
pub mod events;
pub mod extract;
pub mod handlers;
pub mod parse;
pub mod req;
pub mod serde_helpers;
pub mod state;

use autohttps_core::CoreContext;
use axum::http::{HeaderValue, Request, Response};
use axum::routing::{get, post};
use axum::Router;
use handlers::{
    acme, app_info, certificates, dashboard, domains, events as sse, local_ca, settings, sync,
    tasks,
};
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState as ApiState;

/// 构建完整 axum 应用(REST + SSE + SPA)。bin 负责绑定监听。
pub fn app(ctx: CoreContext) -> Router {
    let state = AppState::new(ctx);

    let api = Router::new()
        // --- 全局 / 聚合 ---
        .route("/app-info", get(app_info::get))
        .route("/dashboard", get(dashboard::overview))
        .route("/events", get(sse::stream))
        // --- certificates ---
        .route(
            "/certificates",
            get(certificates::list).post(certificates::create),
        )
        .route(
            "/certificates/{id}",
            get(certificates::get).delete(certificates::delete),
        )
        .route("/certificates/{id}/renew", post(certificates::renew))
        .route("/certificates/{id}/retry", post(certificates::retry))
        .route("/certificates/{id}/revoke", post(certificates::revoke))
        .route("/certificates/{id}/export", get(certificates::export))
        // --- domains ---
        .route("/domains", get(domains::list).post(domains::create))
        .route(
            "/domains/{id}",
            get(domains::get)
                .patch(domains::update)
                .delete(domains::delete),
        )
        // --- settings ---
        .route("/settings", get(settings::get).patch(settings::patch))
        // --- sync(WebDAV 备份)---
        .route(
            "/sync/webdav-config",
            get(sync::get_config)
                .put(sync::put_config)
                .delete(sync::delete_config),
        )
        .route("/sync/test", post(sync::test))
        .route("/sync/backup", post(sync::backup))
        .route("/sync/backups", get(sync::list_backups))
        .route("/sync/restore", post(sync::restore))
        // --- tasks ---
        .route("/tasks", get(tasks::list))
        .route("/tasks/{id}", get(tasks::get))
        .route("/tasks/{id}/logs", get(tasks::logs))
        .route("/tasks/{id}/retry", post(tasks::retry))
        .route("/tasks/{id}/cancel", post(tasks::cancel))
        // --- acme accounts ---
        .route(
            "/acme/accounts",
            get(acme::accounts_list).post(acme::account_create),
        )
        .route(
            "/acme/accounts/{id}",
            get(acme::account_get)
                .patch(acme::account_patch)
                .delete(acme::account_delete),
        )
        .route("/acme/accounts/{id}/retry", post(acme::account_retry))
        // --- acme http01 configs ---
        .route(
            "/acme/http01-configs/{domainId}",
            get(acme::http01_get).put(acme::http01_put),
        )
        // --- acme challenges ---
        .route("/acme/challenges", get(acme::challenges_list))
        .route("/acme/challenges/{id}", get(acme::challenge_get))
        .route(
            "/acme/challenges/{id}/dns-precheck",
            get(acme::dns_precheck),
        )
        .route(
            "/acme/challenges/{id}/confirm",
            post(acme::challenge_confirm),
        )
        .route("/acme/challenges/{id}/retry", post(acme::challenge_retry))
        // --- root CAs ---
        .route("/root-cas", get(local_ca::list).post(local_ca::create))
        .route("/root-cas/import", post(local_ca::import))
        .route("/root-cas/{id}", get(local_ca::get))
        .route("/root-cas/{id}/export", get(local_ca::export))
        .with_state(state);

    Router::new()
        .nest("/api", api)
        // SPA:内嵌前端产物 + client-side 路由 fallback(common §6.1 同源,生产无需 CORS)
        .fallback(embed::spa_handler)
        // 安全响应头:API 与 SPA 同源自托管,收紧 CSP 到 self(SSE 走 connect-src 'self');
        // frame-ancestors 替代 X-Frame-Options 的 DENY(二者并存无害)。
        .layer(axum::middleware::from_fn(security_headers))
        // 开发期放行 Vite dev server 源(ARCHITECTURE §4.3);生产同源不依赖 CORS
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// 为所有响应(API + SPA)附加安全响应头。
async fn security_headers(
    req: Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response<axum::body::Body> {
    let mut resp = next.run(req).await;
    let h = resp.headers_mut();
    h.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; font-src 'self' data:; connect-src 'self'; \
             frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        ),
    );
    h.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        axum::http::header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    h.insert(
        axum::http::header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    resp
}

/// SSE wire 契约(单一定义,导出 TS):事件类型枚举 + 包络 + core `DomainEvent` → 包络的映射。
pub use events::{to_server_event, EventType, ServerEvent};
