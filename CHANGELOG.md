# Changelog

本文件遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 格式,版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added

- 服务层集成测试(内存 SQLite + 临时数据目录):boot 崩溃恢复、证书创建/续签/吊销/重试/删除门控、任务取消回退。
- 领域规则单元测试:证书 10 态状态机判定(`can_renew`/`can_revoke`/`can_retry`/待处理集等)与枚举 wire 值(snake_case)锁定。
- 工具函数单元测试:UUIDv7 生成、RFC3339 解析/回读、`days_until` 边界(未来/过去/取整)。
- boot 序列新增**孤儿密钥材料清扫**:删除 `secrets/` 下不再被任何实体 `*_ref` 引用的 `.age` 文件(多步写中途崩溃的兜底)。
- 全局安全响应头:`Content-Security-Policy`(收紧到 `self`,SSE 经 `connect-src 'self'`)、`X-Content-Type-Options: nosniff`、`X-Frame-Options: DENY`、`Referrer-Policy: no-referrer`。
- 内嵌静态资源缓存策略:Vite hash 产物 `assets/*` 长缓存不可变,`index.html` 不缓存。

### Changed

- 证书创建(证书 + SAN 关联)与删除(任务取消 + 关联清理 + 行删除)的库内多步写纳入事务,失败整体回滚;事件广播与文件清理移至提交之后。
- CI 收紧:`cargo fmt --check` 与 `cargo clippy` 不再 `continue-on-error`,clippy 以 `--all-targets -D warnings` 作为硬门禁。

### Fixed

- 修复全部 clippy `doc_lazy_continuation` 文档警告(clippy 现零警告)。

## [0.1.0-dev] - 2026-07-18

开发预发布(滚动 prerelease,标签形如 `v0.1.0-dev.N`):证书全生命周期管理,桌面 + 服务器双形态共用一套 core/api/前端。

### Added

- **签发方式**
  - ACME 公共 CA(instant-acme):账户注册、HTTP-01(webroot)自动验证、DNS-01 手动挑战(展示 TXT、挂起等待确认、DNS 预检 hickory-resolver)。
  - 自签根 CA(rcgen):创建/导入根 CA、签发内网叶子证书、本地作废记录。
- **证书生命周期**:签发 → 有效 → 即将到期/过期 → 续签/失败重试/吊销/删除/导出(叶子/链/全链/私钥,私钥导出需风险确认);按部署目标(Nginx/Apache/IIS PFX/HAProxy)打包 zip 导出。
- **自动化**:boot 启动即全量扫描 + 崩溃恢复(running→failed 可重试);到期前按策略自动续签;`renewal_failed` 随扫描周期再尝试。
- **实时可见**:dashboard 三指标 + 待处理清单;SSE 全局事件推送(证书/任务/挑战/根 CA 状态变更)。
- **两种运行形态**
  - 桌面(Tauri):800×600 窗口、系统托盘常驻、关窗隐藏、开机自启、托盘红点角标、原生保存对话框导出、单实例。
  - 服务器:守护进程 + 内嵌 SPA Web UI(rust-embed),REST/SSE 统一挂 `/api`。
- **数据安全**:私钥/账户密钥/根 CA 私钥经 age(X25519 + ChaCha20-Poly1305)加密静态存储,库内只存引用;日志脱敏。
- **工程**:SeaORM + SQLite(WAL)持久化;枚举单一定义(ts-rs 投影 TS 契约);CI(fmt/clippy/check/test + 前端构建);Windows portable zip 与 semver 预发布(`vX.Y.Z-dev.N`)工作流。

[Unreleased]: https://github.com/nvrenshiren/autohttps/commits/master
