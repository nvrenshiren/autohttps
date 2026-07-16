# API 契约 · 全局 SSE 事件流(common)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `ARCHITECTURE.md §9`(SSE 推送 + 轮询兜底、**单向 server→client**)· `TECH.md 决策8`(SSE + EventSource + 自动重连)· `TECH.md §1.4`(react-query 配 SSE `invalidate`)· 各 flows 5 台状态机。
> 定位:**全系统唯一的实时通道**。一个全局 stream 端点,各模块的异步状态变更统一经此推送;前端据事件 **invalidate react-query** 缓存后重取。共用约定见 [`conventions.md`](./conventions.md)。

---

## 1. 为什么一个全局流(coherence:一个权威)

- 5 台状态机的实时诉求(证书状态推进、任务进度、DNS-01 `awaiting_manual` 待处理、红点更新)统一走**同一** axum SSE 端点(ARCHITECTURE §9.2);**各模块不各造一套流**。
- **单向**(server→client):MVP 无前端→服务端流式诉求,SSE 足够(WS 备选后置)。
- 触发来源:任务执行器完成一次执行、扫描器推进状态、DNS-01 挑战进入等待、ACME 账户注册完成——服务端据此发事件,前端刷新对应 react-query key。

---

## 2. 端点

```
GET /events        Accept: text/event-stream
```

- 返回 `Content-Type: text/event-stream`,长连接;标准 `EventSource` 消费,浏览器/回环均通,**自动重连**。
- **全局、无过滤参数**:单实例单 operator(D4),前端接收全部事件并按类型映射到 query 失效。
- **心跳**:服务端周期发送注释行(`: keep-alive\n\n`)维持连接。
- **断线续传(可选)**:每条事件带 `id:`(单调递增序号),重连时浏览器自动带 `Last-Event-ID`,服务端**尽力**补发其后事件;不保证不丢——**前端在重连(`onopen`)后应主动全量重取关键列表**兜底(见 §5)。

---

## 3. 事件帧格式

标准 SSE 帧,`event:` 为类型、`data:` 为 JSON 包络:

```
id: 10241
event: certificate_status_changed
data: {"type":"certificate_status_changed","at":"2026-07-16T08:00:00Z","payload":{"certificateId":"018f...","status":"valid"}}
```

统一 JSON 包络:

```ts
interface ServerEvent<T = unknown> {
  type: EventType;   // 与 SSE event: 字段一致(冗余,便于统一 onmessage 处理)
  at: string;        // RFC3339 UTC:事件发生时间
  payload: T;        // 各类型的最小载荷(仅够定位 + 失效,不搬运整实体)
}
```

> **载荷极简原则**:payload 只携带**资源标识 + 关键判别字段**(如新状态),**不搬运整实体**。前端收到后 `invalidate` 对应 query,由 react-query 重新 `GET` 拉取权威数据——避免 SSE 成为第二数据源、与 REST 漂移(呼应 TECH §4 单一真相)。

---

## 4. 事件类型清单(单一定义,导出 TS)

> `EventType` 与各 payload 类型在 `crates/api` 单一定义、经 ts-rs 导出到 `frontend/src/bindings/`;**前端 import,不手写事件名字面量**(同枚举纪律 L1)。状态字段取 TECH §4.3 wire 值。

| # | `type` | 发出模块 | payload | 语义 / 触发 | 前端失效目标(react-query) |
| --- | --- | --- | --- | --- | --- |
| 1 | `certificate_status_changed` | certificates | `{ certificateId, status }` | 证书状态机任一流转(执行器结果 T2–T4/T12–T13/T18–T19、扫描 T6/T10、取消回退 T21–T24) | 证书列表 / 该证书详情 · 域名证书态投影 · dashboard 聚合 |
| 2 | `task_status_changed` | tasks | `{ taskId, certificateId, status }` | 任务状态机流转(TT1 入队 / TT2 开始 / TT3–TT6 终态 / TT7 派生新任务) | 任务列表 / 该任务详情 · 关联证书详情(进行中任务)· dashboard |
| 3 | `task_log_appended` | tasks | `{ taskId, seq }` | 任务执行中新增一条日志(进度) | 该任务日志(`GET /tasks/{id}/logs`,增量拉 `?afterSeq`) |
| 4 | `challenge_status_changed` | acme | `{ challengeId, taskId, domainId, status }` | 挑战状态机流转(CT1–CT10)。**`status=awaiting_manual` 即任务要求的"挑战进入 awaiting_manual"信号**(DNS-01 待手动添加 TXT)——待处理提示 + 红点来源之一 | 验证向导 / 挑战详情 · 任务详情 · dashboard(待处理)|
| 5 | `acme_account_status_changed` | acme | `{ accountId, status }` | ACME 账户状态机流转(AT2/AT3 注册完成 / 失败) | ACME 账户列表 / 详情 |
| 6 | `root_ca_status_changed` | local-ca | `{ rootCaId, status }` | 根 CA 状态机流转(L3 扫描 `active→expired`) | 根 CA 列表 / 详情 · dashboard(链根到期影响提示)|
| 7 | `dashboard_changed` | dashboard(聚合) | `{ pendingCount }` | **红点更新**:待处理集合 / 三指标发生变化时的**粗粒度合并信号**;由 #1/#4/#6 引发聚合变化时一并发出 | dashboard 聚合;**桌面托盘角标**(仅桌面)直接据此更新红点 |

### 4.1 类型与状态取值

- `status` 字段的取值严格取各自状态机 §4.3 wire 值:
  - 证书:`pending_issue`…`revoked`(10 态);任务:`queued`…`cancelled`(5 态);挑战:`pending`…`cancelled`(6 态);账户:`registering`/`registered`/`registration_failed`;根 CA:`active`/`expired`。
- `dashboard_changed.pendingCount` = 待处理证书数(即将到期 + 失败),口径同 `GET /dashboard`(见 dashboard API 文档);仅作红点开关/角标,明细仍由前端重取聚合。

### 4.2 "挑战进入 awaiting_manual" 与红点(coherence 说明)

- DNS-01 挑战 CT3 进入 `awaiting_manual` 时,发 `challenge_status_changed`(`status=awaiting_manual`);它是**手动添加 TXT 待处理提示**的实时来源(acme flows §4.3 / ARCHITECTURE §9)。
- dashboard **三指标 + 证书待处理清单**的口径**严格基于证书状态**(dashboard 权威,DB1);`awaiting_manual` 是 **acme 侧独立的待处理信号**,经 `GET /acme/challenges?status=awaiting_manual` 取明细,不并入 dashboard 三指标计数,避免双口径(详见 dashboard / acme API 文档)。首屏是否并列展示该提示属页面 PRD。

---

## 5. 轮询兜底与消费约定(TECH 决策8 / §1.4)

- **兜底**:`EventSource` 不可用 / 断开且未及时重连时,前端回退到**低频轮询**——即对关键列表(dashboard / 证书 / 任务 / 挑战)启用 react-query `refetchInterval`;SSE 恢复后停轮询。**无需额外轮询端点**,复用既有 `GET`。
- **重连兜底**:`onopen` 重连后主动 `invalidate` dashboard + 当前页关键列表,弥补断线期间可能丢失的事件。
- **zustand 只存 SSE 连接状态**(连接中 / 断开,TECH §1.4);**服务端数据一律走 react-query**,SSE 只触发 `invalidate`,不把 payload 写入客户端缓存作为数据源。

---

## 6. 纪律

- **一个流、一套事件类型**:新增事件类型须经 architect 改 `crates/api` 事件定义 + 同步本表 + 重新导出 TS;模块不得私增流或私发未登记事件。
- **payload 不搬运整实体、不含敏感字段**:仅标识 + 判别字段;密钥材料、`*_ref` 绝不入事件(AR4 / L6)。
- **事件是失效信号,不是数据源**:前端据事件重取 REST 权威数据(单一真相),不以 payload 直接渲染业务数据。
