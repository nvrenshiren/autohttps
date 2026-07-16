---
name: build-layout-notes
description: 工作区构建落位:默认成员排除 desktop;rust-embed 需 frontend/dist 存在;server 默认 8443
metadata:
  type: project
---

三层 crate + 前端的构建要点(踩过的坑,避免重复):

- **默认成员排除 desktop**:根 `Cargo.toml` 的 `default-members = [core, api, server]`,把 `desktop`
  排除。否则 `cargo build` / `cargo run -p server` 会连带拉 Tauri 全套(wry/webview2/windows crates,重)。
  desktop 单独 `cargo check -p desktop`(约 1 分钟首编,通过)。
  **Why:** 里程碑1 server 必须能跑是硬指标,不该被 Tauri 系统依赖拖累。

- **rust-embed 需 `frontend/dist` 目录存在**(编译期):`crates/api/src/embed.rs` `#[folder="../../frontend/dist"]`。
  全新克隆先 `cd frontend && npm run build` 再 `cargo build`;否则 rust-embed 报"folder does not exist"。
  已用 `.gitignore` 例外 + `frontend/dist/.gitkeep` 兜底目录存在(见 .gitignore 末尾三行)。

- **server 默认监听 `127.0.0.1:8443`**(settings 默认);Vite dev 代理 `/api`→8443。改端口用 env
  `AUTOHTTPS_ADDR=host:port`;数据目录 `AUTOHTTPS_DATA_DIR`(默认 `./data`)。REST/SSE 统一挂 `/api`
  前缀(避免与 SPA client 路由如 `/certificates` 冲突);前端 api baseURL = `/api`。

- **时间列存 String(RFC3339)**:SeaORM 实体的时间列用 `String`(非 `TimeDateTimeWithTimeZone`),直接
  等于 wire 表示、避免时区漂移;DB `_overview §1` 的 "TEXT·RFC3339" 即此。`sea_orm(db_type="Text")`
  对 DeriveActiveEnum 可用(1.1 实测通过)。
