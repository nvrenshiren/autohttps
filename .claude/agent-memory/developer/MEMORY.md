# Developer Agent Memory — autohttps

- [枚举 wire 值:含数字变体必须显式 rename](enum-digit-wire-values.md) — serde snake_case 把 `Http01`→`http01`,与契约 `http_01` 不符
- [三层 crate 落位与构建要点](build-layout-notes.md) — 默认成员排除 desktop;rust-embed 需 frontend/dist 存在
- [Windows 自验证踩坑](win-e2e-verification-gotchas.md) — Python 读 UTF-8 用 -X utf8;原生 Python 不认 MSYS /tmp;重建前杀 server.exe
- [instant-acme 0.8 集成](instant-acme-integration.md) — features 要 ring;builder_with_root 信任 Pebble CA;finalize() 取叶子密钥;Pebble 随机有效期
- [Tauri v2 桌面壳集成](tauri-desktop-shell.md) — 托盘句柄可跨线程;壳任务用 axum 的 rt.handle;回环端口固定 38443;自启 Rust 侧驱动+SettingsChanged
