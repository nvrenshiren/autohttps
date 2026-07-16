---
name: reference-baseline-docs
description: 技术基线权威文档位置:仓库根 ARCHITECTURE.md + TECH.md(编码协议 / 枚举单一真相入口)
metadata:
  type: reference
---

autohttps 的**技术基线**是仓库根的两份文件(architect 的 0 号产物,DAG 上游):
- `ARCHITECTURE.md` — 系统架构:三层 crate(core 业务真相 / api 传输契约 / server+desktop 形态宿主)、两形态共享一套前端的传输决策、进程模型、任务队列 + 扫描器落点、启动即检测 + 崩溃恢复落点。
- `TECH.md` — 技术选型总表 + §2 决策清单(10 项已由 orchestrator 于 2026-07-16 全部确认,见 §7-AR7)+ **§3 编码协议(定死不得漂移)** + **§4 枚举/字典单一真相机制**。

**How to apply:**
- 设计任何模块 DB/API 前先读这两份;编码协议(JSON camelCase / 枚举 wire 值 snake_case 照 PRD / limit-offset 分页 / `{error:{code,message}}` / RFC3339 UTC)以 TECH §3 为准,别另立一套。
- **枚举是 architect 唯一变更入口**:全部共享枚举定义于 `crates/core/src/domain/enums.rs`,经 ts-rs 导出到 `frontend/src/bindings/`;新增/改枚举须改 core + 同步 TECH §4.3 表 + 重新导出,developer 不得自行加。
- 库版本核实方式见 [[reference-version-check]]。
