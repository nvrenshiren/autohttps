---
name: reference-db-coherence-anchor
description: DB 层跨模块一致性锚 _overview.md 的位置 + 两条可复用关系约定(soft-reference / 证书枢纽 XOR),API 契约设计须沿用
metadata:
  type: reference
---

autohttps 的 **DB 设计跨模块一致性锚** = `docs/architecture/database/_overview.md`(全局 ER + FK 删除行为总表 + 枚举→§4.3 映射 + 敏感数据落法)。7 模块 DB 文档在同目录 `{模块}.md`,dashboard 无表(纯聚合)。设计 API 契约或改 schema 前先读此锚,保持一致。

**两条本轮确立、易被后续违背的可复用约定**(设计 API 响应/错误码时须沿用):

1. **软引用(soft reference)= 跨父实体生命周期长存的历史/账本**:`tasks.certificate_id`、`challenges.domain_id`、`internal_cert_revocations.certificate_id` 三处**不设 DB 外键级联**,父行硬删除后子行保留原 id;"父已删除"由父表该 id 是否存在判定(依赖 UUIDv7 **不复用**,TECH §3.5)。默认别用 CASCADE/RESTRICT——那会破坏"证书删后任务历史只读保留"(tasks DT3/Q2)等要求。API 侧须反映"证书已删除"标注。

2. **证书枢纽 XOR 不变量**:`certificates` 的 `acme_account_id`(SET NULL)与 `root_ca_id`(RESTRICT)按 `issuance_method` **互斥**——`acme`⇒前者有值后者空,`self_signed`⇒反之(服务层强制,非 DB CHECK)。证书↔域名走唯一 junction `certificate_domains`(SAN 多对多,至多一通配符=服务层不变量)。

**局部属性非 §4.3 枚举**:`acme_accounts.environment`(生产/测试展示标签)、`root_cas.creation_method`(created/imported)是局部字段、**故意未纳入** TECH §4.3 跨端枚举清单(已在文档标注治理路径);若后续前端要强类型消费,才经 architect 走枚举变更入口升格。别误当漏项补进 §4.3。

枚举真相入口与基线见 [[reference-baseline-docs]]。
