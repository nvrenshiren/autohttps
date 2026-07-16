---
name: opcflow-artifact-registration
description: How opcflow tracks/re-registers PRD artifacts — output is create-only, content auto-refreshes on edit, use scan/sync to confirm
metadata:
  type: reference
---

opcflow artifact registration semantics (verified by inspecting `.workbench/workbench.db`, a SQLite store; `artifacts` table has `content_hash` / `approved_hash` / `submitted_at` / `reviewed_at`).

- **`output --role=... --endpoint=common -- <path>` is CREATE-only.** For an already-registered path it errors `✗ 产出文件已存在: <path>` (NOT a gate error). All PRD flow/module docs under `docs/prd/{flows,modules}/*.md` are already registered (endpoint = `common`, module inferred from path).
- **Editing an already-registered artifact in place auto-refreshes its `content_hash` + `updated_at`.** Observed: after editing 12 files, each artifact row's `updated_at` matched the exact order/time I saved edits (untouched files kept their old timestamp). So the workbench sees edited content without any manual re-register step.
- **To confirm the store is in sync after editing, run `scan` then `sync`.** In-sync looks like `扫描完成:...内容刷新 0...` and `对账完成:检查 N,变更 0,失效 0`. `内容刷新 0` means "already refreshed", not "didn't detect".
- `show <id>` is TASK-only; it will say `任务 #<id> 不存在` for an artifact id. Use `artifacts` (table view) or query the DB for artifact state.

**Why this matters:** a task may tell you to "重新登记 via `output`" for edited existing artifacts — that command cannot do it (create-only). Don't fight it or look for a force flag; the edit is already tracked. Just run `scan`/`sync` to confirm 0 drift and report honestly.
