---
name: reference-version-check
description: 本项目未接入 context7 MCP;核实库/版本事实改走 crates.io / npm registry + 本机 cargo/npm
metadata:
  type: reference
---

核实"某 Rust crate / npm 包当前版本 + 维护度"时,**本项目 `.mcp.json` 只配了 opcflow,没有 context7**(全局规则提到 context7,但 architect agent 会话里它不是可调用工具)。

**改用官方 registry 直查**(本机有 cargo 1.95 / node 24 / npm 11):
- crates.io:`curl -s -A "ua (email)" https://crates.io/api/v1/crates/<name>` → JSON 里 `crate.max_stable_version` / `updated_at` / `recent_downloads`。需带 User-Agent,否则被拒。
- npm:`npm view <pkg> version`。

**Why:** 遵"不凭记忆下版本结论",但 context7 在此会话不可用;registry 直查同样是"当前事实"来源,且往往新于模型知识截止。2026-07 实测多项显著新于旧知识(Tauri 2.11、SeaORM 已 1.x 稳定、sqlx 0.9、keyring 4、Vite 8、React 19、TS 7、Zod 4)。

**How to apply:** 任何涉及具体库版本/维护度的选型核实,直接用上面两条命令,别等 context7。选型结论写进 [[reference-baseline-docs]] 指向的 ARCHITECTURE.md / TECH.md。
