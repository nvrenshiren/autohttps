# API 契约 · 总览仪表盘(dashboard)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/dashboard.md §2 功能列表`(A1 指标 / B1 待处理清单 / B2 红点 / C1 跳转 / C2 入口)· `flows/dashboard.md`(聚合口径 §2 / 红点触发 §3.1 / 纯聚合无状态机 §0)· `database/dashboard.md`(无表,只读聚合)· 共用约定 [`common/conventions.md`](./common/conventions.md) · 全局 SSE [`common/events.md`](./common/events.md)。
> **边界**:纯聚合只读;证书状态唯一真相在 certificates、任务概况在 tasks;dashboard **不改任何状态、不落副本**(DD1 / DB2)。
> **不含**:证书 / 任务 / 域名的任何增删改执行(仅只读聚合 + 前端跳转)· 证书状态判定 / 扫描(归 certificates)· 多渠道通知(project §6.2)· project §5 未明示的统计维度(DD3)。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 |
| --- | --- | --- | --- |
| GET | `/dashboard` | 启动首屏聚合:三指标 + 待处理清单 + 红点计数 | A1 · B1 · B2 |

> **C1 待处理项跳转 / C2 常用操作入口 = 前端路由**,非 API 端点:C1 用响应内 `certificateId` / `latestTaskId` 跳证书详情 / 任务详情;C2 跳签发页 / 证书列表 / 域名 / 任务(前端导航)。**红点载体差异**(桌面托盘角标 vs 浏览器内)由 `runMode`(`GET /app-info`)+ SSE `dashboard_changed`(common/events #7)驱动,非本端点返回差异。

---

## 2. 端点详情

### 2.1 `GET /dashboard` — 聚合总览(A1 · B1 · B2)

- 无请求参数;200 → `DashboardOverview`。
- **实时聚合**(database §3):每次按 certificates + tasks 实时聚合,**不落快照表**;前端经 SSE `invalidate` 后重取(见 §5)。
- **口径(引用 flows/dashboard §2 / §3.1,不复述规则)**:
  - `totalCount` = 全部未删除证书(含终态 `revoked`);
  - `expiringSoonCount` = `status = 'expiring_soon'`;
  - `failedCount` = `status IN ('issue_failed','renewal_failed','expired')`;
  - `pendingCount` = `expiringSoonCount + failedCount`(= 待处理清单条数 = 红点触发集大小,三者严格一致,DB1)。
- **待处理清单**:触发集 `status IN ('expired','issue_failed','renewal_failed','expiring_soon')`;**告警级(`expired` / `issue_failed` / `renewal_failed`)排在关注级(`expiring_soon`)之前**(flows §3.1 优先级),服务端已按此排序返回。

---

## 3. 关键 DTO(camelCase)

```ts
interface DashboardOverview {
  metrics: {
    totalCount: number;                    // 全部未删除证书(含 revoked)
    expiringSoonCount: number;             // status='expiring_soon'
    failedCount: number;                   // issue_failed + renewal_failed + expired
  };
  pendingCount: number;                    // = expiringSoonCount + failedCount(红点计数;0 则红点清零)
  pendingItems: PendingCertItem[];         // 待处理清单(告警级优先,服务端已排序)
}
interface PendingCertItem {
  certificateId: string;                   // C1 跳证书详情
  status: CertificateStatus;               // 投影自 certificates(§4.3)
  domains: string[];                       // 关联域名 hostname(经 certificate_domains→domains 携带,DS2)
  issuanceMethod: IssuanceMethod;
  notAfter: string | null;
  daysUntilExpiry: number | null;          // 已过期为负;计算字段(common §1)
  latestTaskId: string | null;             // C1 跳最近任务查失败原因(DS3);无任务则 null
}
```

> **只读投影,不落副本**(DD1 / DB2):全部字段派生自 certificates / tasks / domains,dashboard 不持久化。
> **不引入"级别"新枚举**:告警级 / 关注级由 `status` 依 flows/dashboard §3.1 映射(前端 import 自 bindings),契约层以**已排序的 `pendingItems` + `status`** 表达优先级,守 §4.3 枚举唯一入口(呼应 architect 对枚举不散落的纪律)。
> **`awaiting_manual` 不并入三指标**:DNS-01 挑战 `awaiting_manual` 是 acme 侧独立待处理信号(common/events §4.2),经 `GET /acme/challenges?status=awaiting_manual` 取明细,**不计入** dashboard 三指标(口径严格基于证书状态,避免双口径,DB1)。首屏是否并列展示该提示属页面 PRD。

---

## 4. 错误码清单(snake_case 领域码)

- **无模块专属错误码**:只读聚合、无资源 id、无领域前置条件;仅可能返回全局 `internal_error`(500)。本端点无查询参数,`validation_failed` 不适用。

---

## 5. 状态机与 SSE

- **无独立状态机**(DB2 / flows §0):纯聚合视图,不产生 / 不流转实体。
- **消费相关 SSE 事件触发重取**:`certificate_status_changed`(核心)· `task_status_changed` · `challenge_status_changed`(含 `awaiting_manual`)· `root_ca_status_changed`(链根到期影响)——见 common/events.md;收到即 `invalidate` `/dashboard`。
- **发出 `dashboard_changed`**(common/events #7):聚合的待处理集 / 三指标变化时发**粗粒度红点合并信号**(`{ pendingCount }`),前端据此更新红点;**桌面托盘角标**(仅桌面)直接据此刷新,无需拉全量。

---

## 6. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。三指标口径(flows §2)、红点触发集(§3.1)、聚合来源(DS1–DS4)、一致性约束(指标 = 清单 = 触发集,DB1)均明确;C1 / C2 为前端跳转、无需端点。
