# Developer Agent Memory — autohttps

- [枚举 wire 值:含数字变体必须显式 rename](enum-digit-wire-values.md) — serde snake_case 把 `Http01`→`http01`,与契约 `http_01` 不符
- [三层 crate 落位与构建要点](build-layout-notes.md) — 默认成员排除 desktop;rust-embed 需 frontend/dist 存在
- [Windows 自验证踩坑](win-e2e-verification-gotchas.md) — Python 读 UTF-8 用 -X utf8;openssl 建 x509 用 minimal -config;重建前杀 server.exe
