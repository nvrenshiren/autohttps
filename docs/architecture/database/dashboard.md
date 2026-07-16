# 数据库设计 · 总览仪表盘(dashboard)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(DB)· 端点: app · 撰写: architect
> 依据(approved): `modules/dashboard.md §4 数据来源`(DS1–DS4,全部只读消费)· `flows/dashboard.md`(纯聚合、无独立状态机 DB2/§0)· `TECH.md` / `ARCHITECTURE.md §9`(SSE 推送 + 轮询兜底)。
> 全局 ER 见 [`_overview.md`](./_overview.md)。

---

## 1. 本模块无持久表

dashboard 是**纯聚合、只读视图**,不产生、不拥有、不流转任何核心实体(DD1 / DB2 / flows §0),因此**不新增任何数据库表**。本文件仅登记其聚合来源,便于跨模块一致性核对。

---

## 2. 聚合来源(只读,不建表)

| dashboard 数据 | 来源表(既有) | 聚合口径(引用,不复述) |
| --- | --- | --- |
| 概览三指标(证书总数 / 即将到期数 / 失败数) | `certificates`(certificates 模块) | 全部未删除证书按 `status` 聚合:总数=全行(含终态 `revoked`);即将到期数=`status='expiring_soon'`;失败数=`status IN ('issue_failed','renewal_failed','expired')`(flows/dashboard §2 / DB1) |
| 待处理清单 + 红点 | `certificates` | 触发集=`status IN ('expired','issue_failed','renewal_failed','expiring_soon')`(flows/dashboard §3.1);告警级优先于关注级 |
| 待处理项关联域名 | `certificate_domains`→`domains`(经 certificates 携带,DS2) | 显示"是哪个域名的证书该处理";dashboard 不直接查 domains |
| 关联任务的执行结果与时间 | `tasks`(DS3) | 待处理项(尤失败类)关联其最近一次任务的结果/时间,供"去排查"跳转;明细归 tasks |
| 运行形态(红点载体差异) | 运行时探测(DS4,非持久) | 桌面=托盘角标+窗口内 / 服务器=浏览器内;呈现依据,非持久数据 |

> **一致性约束(供全局核对)**:概览"即将到期数 + 失败数" = 待处理清单条数 = 红点触发集大小(flows/dashboard §2/§3.1 完全一致)。这四类待处理态严格取自 certificates 证书状态机的"需关注/需处理"性质,dashboard 不另立口径、不新增告警态(DB1/DB3)。

---

## 3. 刷新机制(读路径,不涉建表)

- **启动即检测**:certificates 启动扫描推进状态(boot 序列,ARCHITECTURE §7),dashboard 首屏读取扫描后最新态。
- **实时刷新**:SSE 推送(状态变更/任务完成/DNS-01 进入 `awaiting_manual` 时)+ 轮询兜底(决策8 / ARCHITECTURE §9.2);前端 react-query 收到 SSE `invalidate` 后重取聚合。
- dashboard 不自建扫描、不落聚合快照表——每次按上述来源实时聚合(数据量为单实例本地库,直接查询足够)。

---

## 4. 纪律

- **不造表、不落副本**(DD1/DB2):证书状态唯一真相在 certificates,任务概况在 tasks;dashboard 复制任何一份都会制造第二真相源与漂移。
- 聚合口径严格映射 certificates `status` 枚举与 flows/dashboard 规则,不新增统计维度(DD3)。
