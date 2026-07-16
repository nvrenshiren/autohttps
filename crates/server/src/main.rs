//! autohttps 服务器守护进程(形态宿主,ARCHITECTURE §4.1)。
//!
//! boot 序列(§7):建/迁移 SQLite(WAL)→ 崩溃恢复 → 启动 axum(REST + SSE + 内嵌 SPA)。
//! 里程碑1:执行器/扫描器/自动续签打桩;进程起来即可 serve API + SPA、连 SQLite。
//!
//! 环境变量:`AUTOHTTPS_DATA_DIR`(数据目录,默认 `./data`)· `AUTOHTTPS_ADDR`(监听 host:port,
//! 覆盖 settings)· `RUST_LOG`(日志级别)。

use autohttps_core::enums::RunMode;
use autohttps_core::CoreContext;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

fn data_dir() -> PathBuf {
    std::env::var("AUTOHTTPS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,tower_http=info")),
        )
        .init();

    let data_dir = data_dir();
    let db_path = data_dir.join("autohttps.db");
    tracing::info!(db = %db_path.display(), "opening SQLite (WAL)");

    // boot:建/迁移库
    let db = autohttps_core::db::connect(&db_path).await?;
    autohttps_core::db::migrate(&db).await?;

    let ctx = CoreContext::new(
        db,
        data_dir,
        RunMode::Server,
        env!("CARGO_PKG_VERSION").to_string(),
    );

    // boot:崩溃恢复(running→failed 可重试)+ 启动即全量扫描(T6/T10 + L3)+ 依 settings 自动续签
    let recovered = autohttps_core::boot::run(&ctx).await?;
    if recovered > 0 {
        tracing::warn!(recovered_tasks = recovered, "崩溃恢复:遗留 running 任务已置失败(可重试)");
    }

    // 任务执行器(tokio worker):消费持久队列,承接 self_signed 签发/续签/吊销(ACME 执行仍桩)
    autohttps_core::services::executor::spawn(ctx.clone());
    // 扫描调度器(周期任务):到期判定(证书 T6/T10、根 CA L3)+ 自动续签触发
    autohttps_core::scan::spawn(ctx.clone());

    // 监听地址:AUTOHTTPS_ADDR > settings 监听地址:端口 > 默认
    let settings = autohttps_core::services::settings::get_or_init(&ctx).await?;
    let addr = std::env::var("AUTOHTTPS_ADDR").unwrap_or_else(|_| {
        format!(
            "{}:{}",
            settings.listen_address.as_deref().unwrap_or("127.0.0.1"),
            settings.listen_port.unwrap_or(8443)
        )
    });

    let app = autohttps_api::app(ctx);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("autohttps 服务器已启动 → http://{addr}  (Web UI + API)");
    tracing::info!("  · REST/SSE 挂在 /api;SPA 由内嵌产物提供(未构建前端则提示先 npm run build)");

    axum::serve(listener, app).await?;
    Ok(())
}
