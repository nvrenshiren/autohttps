//! autohttps 桌面壳(形态宿主,ARCHITECTURE §4.2)。
//!
//! 方案 A(统一 HTTP/WS):进程内启动**同一** axum 服务,绑定 `127.0.0.1:<回环端口>`(仅回环、机外不可达),
//! Tauri WebView 导航到 `http://127.0.0.1:<port>`(见 tauri.conf.json 的 window.url)。前端与网络层
//! 与服务器形态完全相同;Tauri 只提供原生外壳。
//!
//! 里程碑1:窗口配置(800×600、min 800×600、resizable)在 tauri.conf.json;进程内起 axum + 单实例。
//! 托盘常驻 / 关窗不退出 / 开机自启 / 原生保存对话框(导出)留待实现期(TODO)。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use autohttps_core::enums::RunMode;
use autohttps_core::CoreContext;
use axum::Router;
use std::path::PathBuf;
use tokio::net::TcpListener;

/// 回环监听地址 —— 与 tauri.conf.json 的 window.url 保持一致。仅回环即安全边界(D4)。
const LOOPBACK_ADDR: &str = "127.0.0.1:38443";

fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("AUTOHTTPS_DATA_DIR") {
        return PathBuf::from(d);
    }
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(base).join("autohttps");
    }
    std::env::temp_dir().join("autohttps")
}

/// boot 序列:建/迁移 SQLite → 崩溃恢复 → 绑定回环监听。返回装配好的 Router + Listener。
async fn boot() -> anyhow::Result<(Router, TcpListener)> {
    let dir = data_dir();
    let db = autohttps_core::db::connect(&dir.join("autohttps.db")).await?;
    autohttps_core::db::migrate(&db).await?;

    let ctx = CoreContext::new(
        db,
        dir,
        RunMode::Desktop,
        env!("CARGO_PKG_VERSION").to_string(),
    );
    let recovered = autohttps_core::boot::run(&ctx).await?;
    if recovered > 0 {
        tracing::warn!(recovered_tasks = recovered, "崩溃恢复:遗留 running 任务已置失败(可重试)");
    }

    let listener = TcpListener::bind(LOOPBACK_ADDR).await?;
    Ok((autohttps_api::app(ctx), listener))
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    // 进程内 tokio 运行时承载 axum(回环)+ 任务执行器 + 扫描器(后两者里程碑1 打桩)。
    let rt = tokio::runtime::Runtime::new().expect("创建 tokio 运行时失败");
    let (app, listener) = rt
        .block_on(boot())
        .expect("boot autohttps core(建库/迁移/绑定回环)失败");
    tracing::info!("桌面回环服务已就绪 → http://{LOOPBACK_ADDR}");
    rt.spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error = %e, "回环 axum 服务退出");
        }
    });

    // 运行 Tauri(阻塞主线程);窗口由 tauri.conf.json 声明(url 指向回环服务)。
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            // 已有实例:后续留待聚焦已存在窗口(TODO)。防两份守护抢同一 SQLite。
        }))
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");

    drop(rt);
}
