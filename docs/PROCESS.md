# 项目工作流程

## 契约与设计

- 产品需求、流程和页面契约维护在 `docs/prd/`;API 与数据库约定维护在 `docs/architecture/`;设计系统接入维护在 `docs/design/`。
- 修改行为前先读对应的项目规则和契约,并遵循文档标明的审核状态。`draft` 表示待审核的文档状态,不能仅因代码已有对应实现而视为已批准；上游契约变更按项目工作流登记、送审。
- PRD 定义产品范围和业务规则；API / 数据库文档定义技术契约；`ARCHITECTURE.md` 与 `TECH.md` 说明共享架构和技术基线。README 面向用户，依赖精确版本以 manifest 与锁文件为准。
- 决策记录维护当前有效结论及必要理由；结论变更时同步修订引用文档，并移除已失效或重复内容。
- UI 的品牌、Token 与组件表达以 `D:\Work\dawi\design` 正式规范及 `docs/design/cohesion-adoption.md` 接入说明为准；页面范围与交互以对应 PRD/API 契约为准。

## 实现与复核

- 使用项目工作流时,遵循 `.claude/agents/` 中对应角色的 claim、gate、complete 流程；已移除的 `.opencode/` 配置不再作为项目入口。代码由 Git 追踪,不单独登记为契约产物。
- 常用命令见 `CONTRIBUTING.md`;CI 工作流见 `.github/workflows/ci.yml` 与 `.github/workflows/portable.yml`。本地按改动范围执行适用的格式、类型、静态检查、测试和构建。
- 项目记忆索引位于 `.claude/agent-memory/*/MEMORY.md`;仅记录可复用的结论、理由、验证状态与下一步。
