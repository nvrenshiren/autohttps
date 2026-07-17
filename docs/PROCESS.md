# 开发过程说明

本项目的**契约层**(需求 / 架构 / 设计)与**实现层**(代码)采用了不同的产出与追溯方式,如实记录如下,便于未来读者理解本仓库的产出方式。

## 契约层 —— opcflow 信任流程

`docs/prd`(需求 PRD / 流程 / 模块 / 页面)、`docs/architecture`(数据库 schema / API 契约 / 技术基线)、`docs/design`(设计系统 + 部分 HTML 原型)由分角色 agent(product-manager / architect / designer)逐层产出,经 [opcflow](https://www.npmjs.com/package/@dawipong/opcflow) 的 `submit → approve` 信任流程**逐份人工审批**(截至定稿:共 55 份产物审批通过、0 打回)。这些是代码的上游契约。

其中若干**产品口径决策**(如一证多域 SAN、多根 CA 并存、存储路径只读、取消→证书回退转移、ACME 选型、UI 栈等)在审批过程中显式提请并由 orchestrator 裁决,决策记录 append-only 保留在对应 `docs/` 文件中。

## 实现层 —— 直接构建 + git 追溯

`crates/`(Rust:`core` / `api` / `server` / `desktop`)与 `frontend/`(React)的实现**未走 opcflow 的 `plan → claim → complete` 任务循环**,而是由 developer agent 按**已审批契约**直接构建:每个切片先保证 `cargo check --workspace` + `npm run build` 全绿,再**运行时验证**(起服务 curl / 浏览器 UI 实测,ACME 对本地 Pebble 测试服务器实测)后提交。

**因此,实现的权威、完整记录是 git 提交历史** —— 每个提交信息描述做了什么,并附带当时的验证结论(如"续签后 serial 变、仍 valid"、"DNS-01 awaiting_manual → confirm → valid")。工作台(opcflow)反映的是契约审批,不反映实现任务谱系;两者互补。

> 这是有意的取舍:优先保证功能端到端做完并逐一验证,而非补一套事后的任务台账。git 历史比事后补录的 claim/complete 记录更可信。
