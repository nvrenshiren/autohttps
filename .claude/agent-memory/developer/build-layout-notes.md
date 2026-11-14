---
name: build-layout-notes
description: 工作区构建落位:默认成员排除 desktop;rust-embed 需 frontend/dist 存在;server 默认 8443;Rust MSRV 1.88;CI 覆盖 Linux core/API/server 与 Windows desktop
metadata:
  type: project
---

三层 crate + 前端的构建要点(踩过的坑,避免重复):

- **默认成员排除 desktop**:根 `Cargo.toml` 的 `default-members = [core, api, server]`,把 `desktop`
  排除。否则 `cargo build` / `cargo run -p server` 会连带拉 Tauri 全套(wry/webview2/windows crates,重)。
  desktop 单独 `cargo check -p desktop`(约 1 分钟首编,通过)。
  **Why:** 里程碑1 server 必须能跑是硬指标,不该被 Tauri 系统依赖拖累。

- **rust-embed 需要 `frontend/dist` 目录**:`crates/api/src/embed.rs` 在编译期读取该目录。fresh clone
  尚未构建前端时,`crates/api/build.rs` 会先创建空目录；`.gitignore` 忽略整个 dist。`npm run build`
  生成正式 SPA 产物,不需要 `.gitkeep` 或 Vite 占位插件。需要验证空目录兜底时,删除 `frontend/dist`
  后执行 `cargo clean -p autohttps-api` 与 `cargo build -p server`。

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

- **Rust MSRV 为 1.88**:workspace 四个 package 均以 `rust-version.workspace = true` 继承。用锁定依赖分别验证：
  `cargo +1.88.0 check --locked -p autohttps-core -p autohttps-api -p server -p desktop` 通过；
  `cargo +1.85.0 check` 被锁定依赖拒绝，涉及 `darling 0.23.0`、`image 0.25.10`、`time 0.3.53`、
  `rcgen 0.14.8`、`serde_with 3.21.0` 等声明需要 Rust 1.88 的包，以及 ICU 2.2 需要 1.86 的包。
  更早的 Rust 1.82 还无法解析 `block-buffer 0.12.1` 的 edition 2024 manifest。依赖锁文件变化后，重新以声明的
  MSRV 验证所有生产 package；不要凭 `rust-version` 单字段或稳定版构建通过推断 MSRV。

- **CI 覆盖范围与门禁**:`.github/workflows/ci.yml` 在 Linux 对 `autohttps-core`、`autohttps-api`、`server`
  运行 `cargo fmt --all -- --check`、带 `--locked` 的严格 Clippy、check、test；
  Windows job 对 `desktop` 执行带 `--locked` 的 Clippy、check、test；`.github/workflows/portable.yml` 在 Windows 执行带 `--locked` 的 Release build。
  `CONTRIBUTING.md` 保持相同 crate 集合与命令。所有这些 CI 步骤都是阻断门禁；不得把 fmt/Clippy 当作可忽略提示。

- **async 文件 I/O 与密钥缓存锁**:在线恢复、启动密钥目录清扫、ACME webroot 文件使用 Tokio fs;备份 zip/scrypt/大包解密放进 `spawn_blocking`。
  `SecretStore` 仍同步读写本地小密钥文件,慢速或网络数据目录可能短暂阻塞 Tokio worker,是已知剩余项;不要机械改写所有调用点。
  identity `RwLock` poison fail-closed,读取/写入/失效均返回 `CoreError`;不恢复身份缓存,避免用未知密钥状态继续加解密。

- **已知依赖警告**:严格 Clippy/check/test 当前会提示 `proc-macro-error2 v2.0.1` 的 future-incompat E0365
  (`extern crate proc_macro` 为私有却被重新导出; rust-lang/rust#127909)。它经
  `i18n-embed-fl 0.9.4 → age 0.11.5` 和 `sea-bae 0.2.1 → sea-orm-macros 1.1.20` 两条传递依赖链引入。
  对该 crate 的 `cargo update --dry-run` 没有可更新项；此轮未更换宏/ORM/加密依赖或打本地 patch，避免改变生成宏及运行时依赖行为。
  上游发布修复版本，或上述直接依赖更新且 Clippy、测试通过后，再移除该已知警告记录。
