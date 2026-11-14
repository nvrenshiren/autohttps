# 项目工作流程

## 契约与设计

- 产品需求、流程和页面契约维护在 `docs/prd/`;API 与数据库约定维护在 `docs/architecture/`;设计系统接入维护在 `docs/design/`。
- 修改行为前先读对应的项目规则和契约,并遵循各文档标明的审核状态。上游契约变更按角色规则登记、送审。
- UI 的品牌、token 与组件表达以 `docs/design/cohesion-adoption.md` 和 COHESION 正式规范为准;页面范围与交互以对应 PRD/API 契约为准。

## 实现与复核

- 实现任务遵循 `.claude/agents/` 或 `.opencode/agents/` 中对应角色的 claim、gate、complete 流程。代码由 Git 追踪,不单独登记为契约产物。
- 常用命令见 `CONTRIBUTING.md`;CI 工作流见 `.github/workflows/ci.yml` 与 `.github/workflows/portable.yml`。本地按改动范围执行适用的格式、类型、静态检查、测试和构建。
- 项目记忆索引位于 `.claude/agent-memory/*/MEMORY.md`;仅记录可复用的结论、理由、验证状态与下一步。
