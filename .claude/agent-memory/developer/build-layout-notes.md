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
  fresh clone 未跑 npm build 时目录不存在,proc-macro 展开报"folder does not exist"。
  **真相纠正**:`.gitignore` 虽有 `!/frontend/dist/.gitkeep` 例外,但 `.gitkeep` **从未真正 commit**
  (`git ls-files frontend/dist/` 为空),占位 index.html 也没有——所以旧"靠 .gitkeep 兜底"的说法是错的,
  fresh clone `cargo build` 确实会挂。**现兜底机制 = `crates/api/build.rs`**:编译期(先于本 crate 编译)
  `create_dir_all(../../frontend/dist)` + 写 `.gitkeep`,让 `cargo build` 不依赖先跑前端;不发
  `rerun-if-changed`(用 Cargo 默认:包内文件变动即重跑,保证 `embed.rs` 变更触发 proc-macro 重展开的
  同一次构建里先重建目录)。`frontend/vite.config.ts` 另加 `keepDistPlaceholder` 插件在 `closeBundle`
  写回 `.gitkeep`(vite build 会清空 dist)。验证兜底:`rm -rf frontend/dist && cargo clean -p autohttps-api
  && cargo build -p server` 应自动重建 dist 并编译通过。

- **debug 构建下 rust-embed 运行时从磁盘读 `frontend/dist`(非编译期内嵌)**:验证前端改动**无需重建
  server 二进制**——`npm run build` 后,已在跑的(或缓存链接的)debug server 直接服务新 dist(实测served
  bundle hash `index-XXXX.js` 恰等于 vite 刚产出的 hash;`cargo build -p server` 0.46s 缓存命中未重编)。
  机制:rust-embed 默认在 debug 下按编译期烙进的绝对 `CARGO_MANIFEST_DIR/../../frontend/dist` 路径运行时读盘
  (release 才真内嵌)。**省验证轮次**:自验证只改前端时,`npm run build` + 复用现有 server 即可;不必 kill+rebuild
  (仅当改了 Rust 才需重建)。SPA client 路由(如 `/acme`、`/certificates/:id/challenges`)由 fallback 返回
  index.html(实测 HTTP 200),前端 react-router 接管。

- **server 默认监听 `127.0.0.1:8443`**(settings 默认);Vite dev 代理 `/api`→8443。改端口用 env
  `AUTOHTTPS_ADDR=host:port`;数据目录 `AUTOHTTPS_DATA_DIR`(默认 `./data`)。REST/SSE 统一挂 `/api`
  前缀(避免与 SPA client 路由如 `/certificates` 冲突);前端 api baseURL = `/api`。

- **时间列存 String(RFC3339)**:SeaORM 实体的时间列用 `String`(非 `TimeDateTimeWithTimeZone`),直接
  等于 wire 表示、避免时区漂移;DB `_overview §1` 的 "TEXT·RFC3339" 即此。`sea_orm(db_type="Text")`
  对 DeriveActiveEnum 可用(1.1 实测通过)。
