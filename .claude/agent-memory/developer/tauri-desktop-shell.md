---
name: tauri-desktop-shell
description: crates/desktop Tauri v2 壳集成的非显然要点(托盘跨线程/运行时归属/回环端口/自启驱动/启动验证)
metadata:
  type: feedback
---

crates/desktop 是方案 A 的形态宿主:进程内起同一 axum 回环 + Tauri 原生外壳(托盘/关窗不退出/单实例/自启)。做实/扩展时的非显然坑:

**Why:** 这些点在 ARCHITECTURE §4.2 只锚定"要做什么",不含实现级约束;错了要么编译不过、要么运行期 sqlx panic、要么白页。

**How to apply:**
- **托盘句柄可跨线程更新**:Tauri v2 `tauri::tray::TrayIcon` 被显式 `unsafe impl Send+Sync`,且 `set_icon/set_tooltip` 内部 `run_item_main_thread!` 代理到主线程 → 可安全在 tokio 任务里持 `TrayIcon` 克隆并调用。红点角标就是这么做的(消费 `DomainEvent::DashboardChanged`)。
- **壳侧后台任务必须跑在 axum 的那个 runtime**:SeaORM/sqlx 连接池归属创建它的 runtime;`boot()` 在 `rt.block_on` 里建池,executor/scan 也在 `rt`。壳侧任务(角标/自启)用 `rt.handle().clone()` 派发,**别用 `tauri::async_runtime::spawn`**(另一个 runtime → sqlx "runtime shutdown" 类 panic)。
- **回环端口是固定的 `127.0.0.1:38443`**(常量 `LOOPBACK_ADDR`),与 `tauri.conf.json` 的 `window.url` 硬绑,不是随机端口(任务描述里的"临时端口"实际实现为固定值)。改一处要改两处。
- **开机自启是 Rust 侧驱动**:`app.autolaunch()`(`tauri_plugin_autostart::ManagerExt`)在启动时 + 收到 `SettingsChanged` 时,把 `settings.autostart_enabled` 幂等同步到 OS(HKCU\Run)。前端只经 HTTP 改设置、无 Tauri JS 耦合;为让运行期改动即时生效,core 加了内部事件 `DomainEvent::SettingsChanged`(settings::update 末尾 emit),**不上 SSE wire**(api `to_server_event` 对它返回 None)。
- **setup 闭包给 `&mut App`**:传给 `MenuItem::with_id`/`Menu::with_items`/`TrayIconBuilder::build` 这些 `&M: Manager` 参数时,`&mut App` 实测能编译(coercion 生效),但为让泛型推断干净,顶部 `let app = &*app;` 重借为 `&App` 更稳。
- **启动验证判据(无法截图时)**:进程**穿过 `tauri::Builder…run()` 仍存活 = 窗口+托盘创建成功**——窗口/WebView2 创建失败会让 `.run()` 报错→`.expect()` panic→进程退出。所以"launch 后轮询 `/api/app-info` 拿到 `runMode:desktop` + 进程 alive"即证窗口起来了。
- **cargo build 别接管道再 `&&`**:`cargo build -p x | tail` 的退出码取自 `tail`(恒 0),会掩盖 build 失败;要判 build 成功用 `cargo build -p x; echo $?` 不接管道。收尾杀进程:`taskkill //F //IM desktop.exe`(MSYS 双斜杠)。
