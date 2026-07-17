//! autohttps 桌面壳(形态宿主,ARCHITECTURE §4.2)。
//!
//! 方案 A(统一 HTTP/WS):进程内启动**同一** axum 服务,绑定 `127.0.0.1:<回环端口>`(仅回环、机外不可达),
//! Tauri WebView 导航到 `http://127.0.0.1:<port>`(见 tauri.conf.json 的 window.url)。前端与网络层
//! 与服务器形态完全相同;Tauri 只提供原生外壳。
//!
//! 已做实的壳能力:
//! - 窗口 800×600、min 800×600、resizable(tauri.conf.json);
//! - 系统托盘常驻(菜单:显示窗口 / 退出;左键点击唤出窗口);
//! - **关窗不退出** → 隐藏到托盘(拦截 CloseRequested);
//! - **托盘红点角标**消费 `DashboardChanged`(有待处理证书→红点图标,清零→素图标);
//! - **单实例**(tauri-plugin-single-instance;第二次启动聚焦已存在窗口,防两份守护抢同一 SQLite);
//! - **开机自启**(tauri-plugin-autostart;启动即把 settings.autostartEnabled 同步到 OS,
//!   并订阅 SettingsChanged 即时同步运行期改动)。
//!
//! 导出(证书/根 CA):桌面形态走**原生保存对话框**(tauri-plugin-dialog `save()` 选路径 +
//! tauri-plugin-fs `writeFile` 写盘);服务器形态无此二插件、仍走 webview `<a download>` 兜底。
//! 分流在前端 `frontend/src/lib/download.ts` 按 `/app-info` 的 runMode 判定(方案A 交付通道分流)。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use autohttps_core::enums::RunMode;
use autohttps_core::services::dashboard;
use autohttps_core::{CoreContext, DomainEvent};
use axum::Router;
use std::path::PathBuf;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast::error::RecvError;

/// 回环监听地址 —— 与 tauri.conf.json 的 window.url 保持一致。仅回环即安全边界(D4)。
const LOOPBACK_ADDR: &str = "127.0.0.1:38443";
/// 主窗口 label(与 tauri.conf.json 一致)。
const MAIN_WINDOW: &str = "main";
/// 托盘 id(用于 tray_by_id 检索)。
const TRAY_ID: &str = "main";
/// 托盘素图标源(内嵌 PNG;红点角标在其上叠加绘制)。
const BASE_ICON_PNG: &[u8] = include_bytes!("../icons/32x32.png");

fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("AUTOHTTPS_DATA_DIR") {
        return PathBuf::from(d);
    }
    if let Ok(base) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(base).join("autohttps");
    }
    std::env::temp_dir().join("autohttps")
}

/// boot 序列:建/迁移 SQLite → 崩溃恢复 → 启动执行器/扫描器 → 绑定回环监听。
/// 返回装配好的 Router + Listener + `ctx`(壳侧订阅事件、读设置用,与 api 共享同一 core 上下文)。
async fn boot() -> anyhow::Result<(Router, TcpListener, CoreContext)> {
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

    // 任务执行器(tokio worker):消费持久队列,承接签发/续签/吊销。
    autohttps_core::services::executor::spawn(ctx.clone());
    // 扫描调度器(周期任务):到期判定(证书 T6/T10、根 CA L3)+ 自动续签触发。
    autohttps_core::scan::spawn(ctx.clone());

    let listener = TcpListener::bind(LOOPBACK_ADDR).await?;
    Ok((autohttps_api::app(ctx.clone()), listener, ctx))
}

/// 32×32 素图标 RGBA + 叠加红点后的 RGBA(row-major, top→bottom)。启动一次性算好,更新角标时克隆。
struct TrayIcons {
    plain: Vec<u8>,
    dotted: Vec<u8>,
    width: u32,
    height: u32,
}

impl TrayIcons {
    fn load() -> anyhow::Result<Self> {
        let base = Image::from_bytes(BASE_ICON_PNG)?;
        let (width, height) = (base.width(), base.height());
        let plain = base.rgba().to_vec();
        let mut dotted = plain.clone();
        draw_red_dot(&mut dotted, width, height);
        Ok(Self { plain, dotted, width, height })
    }

    fn image(&self, dotted: bool) -> Image<'static> {
        let rgba = if dotted { self.dotted.clone() } else { self.plain.clone() };
        Image::new_owned(rgba, self.width, self.height)
    }
}

/// 在图标右上角叠加一个红点角标(红填充 + 白描边,提升在各种托盘底色下的辨识度)。
fn draw_red_dot(rgba: &mut [u8], w: u32, h: u32) {
    let (wi, hi) = (w as i32, h as i32);
    let r = (wi.min(hi) / 5).max(4); // 红点半径(32px → ~6px)
    let ring = r + 1; // 白描边外半径
    let cx = wi - r - 1; // 右上角
    let cy = r + 1;
    for y in (cy - ring)..=(cy + ring) {
        for x in (cx - ring)..=(cx + ring) {
            if x < 0 || y < 0 || x >= wi || y >= hi {
                continue;
            }
            let (dx, dy) = (x - cx, y - cy);
            let d2 = dx * dx + dy * dy;
            let idx = ((y * wi + x) * 4) as usize;
            if idx + 3 >= rgba.len() {
                continue;
            }
            if d2 <= r * r {
                // red-600 (#DC2626) 不透明
                rgba[idx] = 0xDC;
                rgba[idx + 1] = 0x26;
                rgba[idx + 2] = 0x26;
                rgba[idx + 3] = 0xFF;
            } else if d2 <= ring * ring {
                // 白描边
                rgba[idx] = 0xFF;
                rgba[idx + 1] = 0xFF;
                rgba[idx + 2] = 0xFF;
                rgba[idx + 3] = 0xFF;
            }
        }
    }
}

/// 唤出并聚焦主窗口(托盘菜单/左键/单实例二次启动共用)。
fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 按待处理数切换托盘图标(红点/素)+ tooltip。set_icon/set_tooltip 由 Tauri 内部代理到主线程,
/// 可安全从任意线程调用(TrayIcon 被标记 Send+Sync)。
fn apply_badge(tray: &TrayIcon, icons: &TrayIcons, pending: i64) {
    let _ = tray.set_icon(Some(icons.image(pending > 0)));
    let tip = if pending > 0 {
        format!("autohttps · {pending} 项待处理")
    } else {
        "autohttps".to_string()
    };
    let _ = tray.set_tooltip(Some(tip));
}

/// 把 settings.autostartEnabled 同步到 OS 开机自启(幂等:仅在与当前 OS 状态不一致时改)。
async fn reconcile_autostart(app: &AppHandle, ctx: &CoreContext) {
    let desired = match autohttps_core::services::settings::get_or_init(ctx).await {
        Ok(s) => s.autostart_enabled.unwrap_or(false),
        Err(e) => {
            tracing::warn!(error = %e, "读取 autostart 设置失败,跳过同步");
            return;
        }
    };
    let mgr = app.autolaunch();
    match mgr.is_enabled() {
        Ok(cur) if cur == desired => {}
        Ok(_) => {
            let res = if desired { mgr.enable() } else { mgr.disable() };
            match res {
                Ok(()) => tracing::info!(desired, "已同步开机自启到 OS"),
                Err(e) => tracing::warn!(error = %e, desired, "同步开机自启失败"),
            }
        }
        Err(e) => tracing::warn!(error = %e, "查询开机自启状态失败"),
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    // 进程内 tokio 运行时承载 axum(回环)+ 任务执行器 + 扫描调度器 + 壳侧事件消费。
    // 所有 DB 访问都在这一个运行时上(SeaORM/sqlx 连接池的归属运行时),壳侧任务经 rt.handle 派发,
    // 不用 Tauri 自带运行时,避免"连接池建于 rt、任务跑在另一 runtime"的 sqlx 陷阱。
    let rt = tokio::runtime::Runtime::new().expect("创建 tokio 运行时失败");
    let (app_router, listener, ctx) = rt
        .block_on(boot())
        .expect("boot autohttps core(建库/迁移/绑定回环)失败");
    tracing::info!("桌面回环服务已就绪 → http://{LOOPBACK_ADDR}");
    rt.spawn(async move {
        if let Err(e) = axum::serve(listener, app_router).await {
            tracing::error!(error = %e, "回环 axum 服务退出");
        }
    });

    let icons = TrayIcons::load().expect("加载托盘图标失败");
    let rt_handle = rt.handle().clone();

    tauri::Builder::default()
        // 单实例最先注册:第二次启动 → 聚焦已存在窗口(防两份守护抢同一 SQLite)。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        // 开机自启:Rust 侧驱动(app.autolaunch());启动后按 settings 同步。
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        // 原生保存对话框 + 文件写入:桌面导出走 save() 选路径 + writeFile 写盘(前端 lib/download.ts
        // 按 runMode=desktop 分流)。权限/scope 见 capabilities/default.json(远程回环 URL 授权)。
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(move |app| {
            // setup 给的是 &mut App;下面各 Manager 取用都只需不可变引用,重借为 &App 让泛型 &M 推断干净。
            let app = &*app;
            // ---- 系统托盘 + 菜单 ----
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(icons.image(false))
                .tooltip("autohttps")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键抬起 → 唤出窗口(菜单走右键)。
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // ---- 壳侧后台任务:红点角标(DashboardChanged)+ 开机自启同步(SettingsChanged)----
            // 先订阅再取初始态,避免初始读取与订阅之间漏掉事件。
            let ctx_task = ctx.clone();
            let app_handle = app.handle().clone();
            rt_handle.spawn(async move {
                let mut rx = ctx_task.events.subscribe();
                // 初始:按当前待处理数点亮角标 + 同步开机自启。
                let initial = dashboard::pending_count(&ctx_task).await.unwrap_or(0) as i64;
                apply_badge(&tray, &icons, initial);
                reconcile_autostart(&app_handle, &ctx_task).await;
                loop {
                    match rx.recv().await {
                        Ok(DomainEvent::DashboardChanged { pending_count }) => {
                            apply_badge(&tray, &icons, pending_count);
                        }
                        Ok(DomainEvent::SettingsChanged) => {
                            reconcile_autostart(&app_handle, &ctx_task).await;
                        }
                        Ok(_) => {}
                        Err(RecvError::Lagged(_)) => {} // 落后:忽略,后续事件仍会拉平状态
                        Err(RecvError::Closed) => break,
                    }
                }
            });

            Ok(())
        })
        // 关窗不退出:拦截主窗口 CloseRequested → 隐藏到托盘。
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN_WINDOW {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用失败");

    drop(rt);
}
