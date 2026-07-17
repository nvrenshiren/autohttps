# 贡献指南

感谢有兴趣参与 autohttps。本文说明开发环境、提交流程与代码规范。

## 开发环境

前置:Rust(1.9x,含 `rustfmt`/`clippy`)、Node.js 20+。

```bash
# 前端(产物内嵌进可执行文件)
cd frontend && npm install && npm run build && cd ..

# 服务器形态
cargo run -p server            # → http://127.0.0.1:8443

# 桌面形态(Tauri)
cargo run -p desktop
```

前端热更新开发:`cargo run -p server` 起后端,另开 `cd frontend && npm run dev`(已配 proxy `/api` → 后端)。

本地测 ACME 见 [README](README.md) 的 Pebble 一节。

## 代码规范

- **Rust**:`cargo fmt --all` 格式化;`cargo clippy` 无告警;`cargo check --workspace` 通过。
- **前端**:`npm run build`(含 `tsc --noEmit`)通过;遵循 Prettier(`.prettierrc`)。
- **契约优先**:领域枚举/状态机的单一真相在 `crates/core`;敏感数据只存 `*_ref`、绝不明文入库或入日志;DTO 不暴露密钥字段。
- 不引入需求未明示的功能(批量操作、额外统计等);改动对齐 `docs/` 下的契约。

## 提交与 PR

1. 从 `main` 切分支开发(`feat/…`、`fix/…`)。
2. **提交信息**:祈使句、简洁描述实际改动(如 `Add DNS-01 manual challenge flow`);一个提交聚焦一件事。
3. 提交前本地跑通:`cargo check --workspace`、`cargo test`、`cd frontend && npm run build`。
4. 开 PR,填写 PR 模板的测试 checklist;CI(见 `.github/workflows/ci.yml`)须通过。
5. 涉及行为变更请更新对应文档/README。

## 目录速览

`crates/core`(业务真相)· `crates/api`(REST + SSE 契约)· `crates/server`/`crates/desktop`(两形态宿主)· `frontend`(React SPA)· `docs`(PRD / 架构 / 设计契约)。
