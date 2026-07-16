# API 契约 · 任务与历史(tasks)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/tasks.md §2 功能列表`(A1/A2 查看、B1 重试、B2 取消;C1–C4 系统行为)· `flows/tasks.md`(任务状态机 5 态 TT1–TT7 · 类型 issue/renew/revoke · 触发 manual/auto/cleanup · 重试链 DT1 · 删除保留 DT3 · 尽力取消 DT2)· `database/tasks.md` · 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:任务是"对某证书的一次执行单元",据结果**驱动**证书状态(§4),但不改写/复述证书状态机;不判到期、不持续签策略/重试参数(归 certificates/settings)。
> **不含**:批量重试/取消、跨证书操作审计视图、手动新建/编辑任务(任务只由 certificates 触发或对失败任务重试派生,DEC5)。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 | 映射转移 |
| --- | --- | --- | --- | --- |
| GET | `/tasks` | 任务列表(队列+历史统一,筛选/分页) | A1 | — |
| GET | `/tasks/{id}` | 任务详情(含重试链) | A2 | — |
| GET | `/tasks/{id}/logs` | 任务执行日志(有序,支持增量) | A2 | — |
| POST | `/tasks/{id}/retry` | 手动重试失败任务(派生新任务) | B1 | TT7 |
| POST | `/tasks/{id}/cancel` | 取消排队/执行中任务 | B2 | TT5·TT6 |

> **系统行为无端点**:C1 入队调度、C2 进程中断恢复(boot 序列)、C3 自动续签承接、C4 证书删除清理——均由 core 服务/执行器驱动,非交互端点(certificates/扫描器触发)。
> **"队列"即列表视图**:`GET /tasks?status=queued,running` 即队列;队列与历史同表(DEC1)。

---

## 2. 端点详情

### 2.1 `GET /tasks` — 列表(A1)

- **过滤**:`taskType`(`issue`|`renew`|`revoke`)· `status`(任务 5 态,可多值;`queued,running` 即队列)· `certificateId`· `trigger`(`manual`|`auto`|`cleanup`)· `dateFrom`/`dateTo`(按 `queuedAt` 区间,RFC3339)。
- **排序**:`queuedAt`(默认 `desc`)· `finishedAt`。
- **分页**:必备(历史只增,tasks §4 数据量注)。
- 响应 `{ items: TaskSummary[], page, pageSize, total }`。

### 2.2 `GET /tasks/{id}` — 详情(A2)

- 200 → `TaskDetail`(含重试链:`parentTaskId` + 反查 `childTaskIds`、`attemptNumber`;关联证书当前态投影 + "证书已删除"标注)。不存在 → `404 task_not_found`。

### 2.3 `GET /tasks/{id}/logs` — 执行日志(A2)

- 按 `seq` 升序返回 `TaskLogEntry[]`;支持 `?afterSeq=<n>` 增量拉取(配合 SSE `task_log_appended` 做进度追加)。
- 可选分页;日志脱敏(不含密钥,AR4/L6)。响应 `{ items: TaskLogEntry[], … }`(afterSeq 模式可返回裸数组增量,统一用分页包络亦可——实现期定,契约取分页包络)。

### 2.4 `POST /tasks/{id}/retry` — 手动重试(B1,TT7)

- 仅 `failed` 任务可重试 → **派生新任务**(同类型、同证书、`trigger=manual`、`attemptNumber+1`、`parentTaskId=原任务`),原失败任务保持 `failed`。新任务 `queued`(**202**),经调度驱动证书告警态→进行中态(`issue_failed→issuing` T5 / `renewal_failed→renewing` T14 / 或对已回退证书重新吊销)。
- 非 `failed` → `409 task_not_retryable`;关联证书已删除 → `409 certificate_deleted`(重试前校验证书仍存在,DB §2.3,避免对已删证书误触发)。

### 2.5 `POST /tasks/{id}/cancel` — 取消(B2,TT5/TT6)

- 仅 `queued`/`running` 可取消 → `cancelled`(`queued` 直接取消 TT5;`running` **尽力而为** TT6,在途 CA 操作可能仍生效,由 certificates 下次扫描据实校正,DT2)。**202**(running)/ 200(queued)。
- **驱动证书回退**(证书状态机唯一真相,本模块只触发):取消签发首签 → 证书 `pending_issue/issuing → issue_failed`(T21/T22);取消续签 → 回退发起前态(T23);取消吊销 → 回退发起前态(T24)。
- 终态任务(`succeeded`/`failed`/`cancelled`)→ `409 task_not_cancellable`。

---

## 3. 关键 DTO(camelCase)

```ts
interface TaskSummary {
  id: string;
  certificateId: string;
  certificateDeleted: boolean;         // 计算:certificates 中该 id 是否已不存在(软引用,DT3/Q2)
  certificateDomains: string[] | null; // 关联证书的 hostname(证书存在时展示;已删除为 null)
  taskType: TaskType;                  // §4.3:issue | renew | revoke
  trigger: TaskTrigger;                // §4.3:manual | auto | cleanup
  status: TaskStatus;                  // §4.3:queued | running | succeeded | failed | cancelled
  attemptNumber: number;
  queuedAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  resultSummary: string | null;
  failureReason: string | null;        // 失败摘要(完整过程见 logs)
}
interface TaskDetail extends TaskSummary {
  parentTaskId: string | null;         // 重试链:由哪个失败任务派生
  childTaskIds: string[];              // 反查:由本任务重试派生的后继任务
  certificate: TaskCertificateRef | null; // 关联证书当前态投影(已删除为 null)
  createdAt: string;
  updatedAt: string;
}
interface TaskCertificateRef { id: string; status: CertificateStatus; domains: string[] }

interface TaskLogEntry {
  id: string;
  taskId: string;
  seq: number;                         // 任务内有序
  loggedAt: string;                    // RFC3339
  level: string;                       // info | warn | error(与 tracing 对齐;局部属性,非 §4.3 枚举)
  message: string;                     // 脱敏:绝不含密钥材料(AR4/L6)
}
```

> **软引用与"证书已删除"**:`certificateId` 保留原值,`certificateDeleted` 经 `LEFT JOIN certificates` 判定(UUIDv7 不复用,不误配)。历史任务只增留痕、证书删除不级联(DT3/Q2)。日志 `message` 脱敏。

---

## 4. 任务→证书状态联动(flows/tasks §4;证书状态机唯一真相,不复述)

| 任务事件 | 驱动证书转移 | 端点 |
| --- | --- | --- |
| issue 入队/开始/成功/失败 | T1/T2/T3/T4 | `POST /certificates`(入队);其余执行器 |
| issue 手动重试 | T5 | `POST /tasks/{id}/retry` 或 `POST /certificates/{id}/retry` |
| renew 开始 | T7/T9/T14/T17/T20(视触发源) | `POST /certificates/{id}/renew|retry` / 自动扫描 |
| renew 成功/失败 | T12/T13 | 执行器 |
| revoke 开始/成功/失败 | T8·T11·T16 / T18 / T19 | `POST /certificates/{id}/revoke`;其余执行器 |
| 取消(queued/running) | T21–T24(回退) | `POST /tasks/{id}/cancel` |

> **重试双入口**:`POST /tasks/{id}/retry`(任务视角,tasks B1)与 `POST /certificates/{id}/retry`(证书视角,certificates B2/C3)**收敛于同一 core 服务操作**(派生新任务 + 驱动证书态),非双真相;前者针对具体失败任务、后者针对证书当前失败动作。取消同理:进行中态证书的取消**只在 tasks 侧**(certificates §2 无取消操作),驱动 T21–T24。

## 5. 状态机 → 端点映射(flows/tasks §3.3)

| 转移 | 触发端点 / 来源 |
| --- | --- |
| TT1 排队(入队) | certificates 触发(`POST /certificates` 或 renew/revoke/retry;无 tasks 直建端点,DEC5) |
| TT2 排队→执行中 | 调度器(无端点) |
| TT3/TT4 执行中→成功/失败 | 执行器(无端点;经 SSE) |
| TT5 排队→已取消 · TT6 执行中→已取消 | `POST /tasks/{id}/cancel` |
| TT7 失败→(派生)排队 | `POST /tasks/{id}/retry`(或 certificates 自动再尝试:扫描触发,无端点) |

## 6. 本模块 SSE 事件(见 [`common/events.md`](./common/events.md))

- `task_status_changed { taskId, certificateId, status }` — 任务流转(入队/开始/终态/派生)。前端失效任务列表/详情、关联证书详情(`activeTaskId`)、dashboard。
- `task_log_appended { taskId, seq }` — 执行中新增日志。前端增量拉 `GET /tasks/{id}/logs?afterSeq=` 做进度。

## 7. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。任务状态机、重试链(DT1)、软引用只读保留(DT3/Q2)、尽力取消(DT2)、不持策略/重试参数(DT5)、取消→证书回退(Q1→T21–T24)均已裁决且明确。任务只由 certificates 触发/失败重试派生(DEC5),故无手动建任务端点。
