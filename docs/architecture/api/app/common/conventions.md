# API 契约 · 跨模块共用约定(common)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `TECH.md §3 编码协议`(REST 风格 / camelCase / 枚举 snake_case / limit-offset 分页 / 错误包络 / RFC3339 UTC)· `TECH.md §4.3 枚举` · `ARCHITECTURE.md §3`(方案 A 统一 HTTP/WS)· `ARCHITECTURE.md §9 / §11`(SSE 推送 + 轮询兜底;D4 无鉴权;D3 止于导出)· 8 份 DB 文档(实体 → DTO)。
> 本文件是**全部 API 契约文档的共用锚**:错误包络、HTTP 状态映射、分页/排序/过滤、无鉴权模型、导出交付形态差异统一在此;各模块 API 文档服从本约定。全局 SSE 事件流契约见 [`events.md`](./events.md)。

---

## 0. 传输总则(承 ARCHITECTURE §3 方案 A)

- 前端**始终且只**经 **HTTP(REST)+ SSE**(推送)与后端通信;两形态(桌面回环 / 服务器)挂载**同一** axum Router,契约**只定义一次**。
- **无 Tauri IPC 数据通道**:桌面的原生能力(托盘、原生保存对话框)由外壳承接,不承载业务数据读写(见 §5 导出)。
- 所有 REST 端点只有一个端点面 `app`,契约落 `docs/architecture/api/app/{模块}.md`,跨模块共用落 `common/`。
- DTO 由 `crates/api` 定义、经 **ts-rs** 派生 TS 到 `frontend/src/bindings/`;**枚举严禁前端手写字面量**,一律 import 自 bindings(TECH §4 / L1)。本文各 DTO 以 TS 形态示意,真相在 Rust。

---

## 1. 命名与序列化(TECH §3.2,定死)

| 面 | 约定 |
| --- | --- |
| JSON 字段名 | **camelCase**(serde `rename_all="camelCase"`) |
| 枚举 wire 值 | **snake_case**,严格等于 TECH §4.3 标识(如 `pending_issue` / `awaiting_manual` / `self_signed` / `http_01`);不 camelCase 化、不翻译、不缩写 |
| 路由资源 | 复数小写,多词 `kebab-case`(`/certificates` `/domains` `/acme/accounts` `/root-cas` `/tasks` `/settings` `/dashboard`) |
| 查询参数 | camelCase |
| 时间 | wire 一律 **RFC3339 / ISO-8601 UTC 字符串**(如 `2026-07-16T08:00:00Z`);请求与响应同此 |
| 实体 ID | UUIDv7 文本、对外不透明、稳定、不复用(TECH §3.5 / 决策10) |

> 计算字段(如 `daysUntilExpiry`)由服务端相对**服务器当前 UTC 时刻**算出;已过期为负、无有效期(未签发)为 `null`。

---

## 2. HTTP 方法语义(TECH §3.1)

| 方法 | 用途 |
| --- | --- |
| `GET` | 查询(列表 / 详情 / 聚合 / 导出下载);无副作用 |
| `POST /{资源}` | 创建资源(如 `POST /certificates` 发起签发、`POST /root-cas` 创建根 CA) |
| `POST /{资源}/{id}/{动作}` | 对已存在资源触发**领域动作**(签发 / 续签 / 吊销 / 重试 / 取消等状态机动作),不硬套 CRUD |
| `PATCH /{资源}/{id}` | 局部修改可变字段(如域名分组 / 备注、设置项) |
| `DELETE /{资源}/{id}` | 删除资源 |

- **动作端点一律 POST 子资源**,请求体多为空或携带少量参数;响应返回受影响资源的最新表示或 `202 Accepted`(异步动作,结果经 SSE 回推)。
- **同步 vs 异步**:纯本地即时操作(创建域名 / 创建根 CA / 改设置)同步返回最终态;涉及在线 CA 交互的动作(签发 / 续签 / 吊销 / ACME 账户注册)**异步**——端点仅登记/入队并返回 `202` + 当前进行中态,终态经全局 SSE 推送(见 [`events.md`](./events.md))。

---

## 3. 分页 / 排序 / 过滤(TECH §3.3,定死)

### 3.1 分页(所有列表端点统一)

- 请求:`?page=<从 1 起,默认 1>&pageSize=<默认 20,上限 100>`。`pageSize` 超上限被钳制为 100;`page` 越界返回空 `items`。
- 响应包络(定死):

```json
{ "items": [ /* 该页元素 */ ], "page": 1, "pageSize": 20, "total": 137 }
```

- 风格为 **limit/offset(page/pageSize)**;`total` 为过滤后总数。

### 3.2 排序

- `?sort=<字段>&order=<asc|desc>`;`order` 默认 `desc`(时间类)/ 由各模块声明默认。
- **可排序字段白名单**由各模块 API 文档声明;未在白名单内的 `sort` 值 → `400 validation_failed`。

### 3.3 过滤

- 过滤参数 camelCase,由各模块声明(如 certificates 的 `status` / `issuanceMethod` / `domain`;tasks 的 `taskType` / `status` / `certificateId` / `dateFrom` / `dateTo`)。
- 枚举型过滤值取 TECH §4.3 wire 值;非法枚举值 → `400 validation_failed`。
- 关键字过滤(如 `domain` / `hostname`)为**子串包含**(大小写不敏感),语义由各模块声明。

---

## 4. 错误包络与 HTTP 状态映射(TECH §3.4,定死)

### 4.1 错误响应包络

```json
{ "error": { "code": "cert_not_found", "message": "证书不存在", "details": { "id": "018f..." } } }
```

- `code`:**稳定 snake_case 领域错误码**,在 core 单一定义(枚举)并经 ts-rs 导出;前端按 `code` 分支处置。
- `message`:人读信息,**仅供展示、可变**,不作分支依据。
- `details`:可选对象,携带定位上下文(如 `{ field }` / `{ id }` / `{ currentStatus, action }`)。

### 4.2 HTTP 状态映射(统一口径)

| 状态 | 语义 | 典型 `code` |
| --- | --- | --- |
| `200` | 成功(含同步动作完成、导出下载) | — |
| `201` | 资源创建成功(`POST /{资源}` 同步创建,如根 CA / 域名) | — |
| `202` | 已受理、异步执行中(签发 / 续签 / 吊销 / 账户注册,终态经 SSE 回推) | — |
| `204` | 成功无响应体(部分 `DELETE`) | — |
| `400` | **入参非法**——请求体 / 查询参数结构错误、缺必填、类型错、非法枚举值、非法 `sort` | `validation_failed` |
| `404` | **资源不存在** | `*_not_found`(如 `cert_not_found`) |
| `409` | **状态冲突 / 引用冲突**——对进行中态动作、被引用不可删、非法状态迁移、终态不可再动作 | `cert_in_progress_cannot_delete` `domain_has_certificates` `invalid_cert_state` `task_not_cancellable` |
| `422` | **业务规则拒绝**——结构合法但违反领域前置条件 / 不变量 | `wildcard_requires_dns01` `root_ca_expired` `acme_account_not_registered` `key_export_not_acknowledged` |
| `500` | 内部错误 | `internal_error` |

> **400 vs 422 判别**:结构/类型层面的错误(能被 schema 校验拦截)= 400 `validation_failed`;结构合法但违反领域规则(需查库 / 查状态机才能判定)= 422 领域码。
> **409 vs 422 判别**:与**当前状态机态 / 引用关系**冲突(同一请求换个时机可能成功)= 409;与**输入内容本身**的领域不变量冲突(换时机仍失败)= 422。

### 4.3 全局错误码(所有模块可用)

| code | HTTP | 含义 |
| --- | --- | --- |
| `validation_failed` | 400 | 请求体 / 查询参数结构或类型非法;`details` 可含 `{ field, reason }` |
| `not_found` | 404 | 通用兜底(优先用模块专属 `*_not_found`) |
| `internal_error` | 500 | 未预期的内部错误(敏感细节不外泄,详见服务端日志) |

> 各模块**领域错误码清单**在各自 API 文档 §"错误码"逐一登记;**全局 snake_case 唯一**——同一 `code` 不得在不同模块承载**不同**语义。跨模块**同义共享**的规则码(如 `wildcard_requires_dns01`、`root_ca_expired`、`acme_account_not_registered`)在相关模块均登记并注明"共享规则,单一语义"。

---

## 5. 导出交付:同一 API、不同通道(D3 / ARCHITECTURE §4.2)

证书 / 根 CA 导出是**专用二进制下载端点**(非 JSON),两形态**共用同一 HTTP 端点**,仅**交付通道**按运行形态分流:

| 端点 | 内容 | 私钥门控 |
| --- | --- | --- |
| `GET /certificates/{id}/export?parts=<…>` | 叶子证书 / 证书链 / 完整链 / **私钥**(可选组合) | 含私钥须 `acknowledgeKeyExport=true`,否则 `422 key_export_not_acknowledged` |
| `GET /root-cas/{id}/export` | 根 CA **公开证书**(PEM);**MVP 不含私钥**(local-ca LC4) | 无(公开材料,无需门控) |

**响应形态**(端点本身与形态无关):
- `Content-Type: application/x-pem-file`(单一 PEM)或 `application/octet-stream`(多部件打包);
- `Content-Disposition: attachment; filename="<资源名>-<部件>.pem"`。

**交付通道差异(前端按运行形态标志分流,见 §6):**
- **服务器形态**:浏览器识别 `Content-Disposition: attachment` → **浏览器下载**到下载目录。
- **桌面形态**:前端(runMode=`desktop`)以 `fetch` 取回响应 blob → 交 **Tauri 原生保存对话框**(tauri-plugin-dialog + -fs)写入用户选定路径。**同一后端导出端点,不同交付通道**;后端不感知形态、不返回不同响应。

**导出前置(状态门控)**:仅"本地已存在证书文件"的证书可导出(certificates flows §2.4);对 `pending_issue` / `issuing` / `issue_failed` 证书导出 → `409 cert_not_exportable`。

**私钥敏感边界**:导出端点是 `*_ref` 引用密钥材料的**唯一对外读取口**;库内绝不返回 `private_key_ref` / `account_key_ref` 本身,DTO 亦绝不含密钥字段(AR4 / L6)。私钥导出经风险确认(UI 提示 + `acknowledgeKeyExport=true` 编码已确认);根 CA 私钥永不导出(LC4)。

---

## 6. 无鉴权模型(D4)与运行形态标志

### 6.1 无鉴权(ARCHITECTURE §11 / project §9-D4)

- **无登录 / 会话 / 多用户 / 权限**:MVP 不做应用层鉴权,信任 = **网络可达性**,安全由部署边界保障。
- 所有端点**无 `Authorization` 头、无鉴权中间件、无登录/登出/用户/会话端点**。
- **桌面形态**:axum 仅绑 `127.0.0.1:<临时端口>`,机外不可达,回环边界即安全边界。
- **服务器形态**:绑 settings 监听地址:端口;默认仅本机 / 可信内网;设为对外可达由 settings 界面提示公网暴露风险(roles §3),契约层不加鉴权。
- **CORS**:生产为同源(SPA 经 rust-embed 内嵌、与 API 同源,无需 CORS);仅开发期放行 Vite dev server 源(ARCHITECTURE §4.3)。

### 6.2 运行形态标志(TECH §3.6)

- `GET /app-info` → `{ "runMode": "desktop" | "server", "appVersion": "…" }`。
- `runMode` 为**运行载体探测的运行时事实**(settings DS5 / SF4),**非持久配置、不可切换、不落库**(settings DB §2.3)。
- 前端据 `runMode` 做**仅桌面 / 仅服务器**显隐(如设置页 autostart vs 监听地址;导出交付通道分流)。该值 app 级跨页共用,由本端点提供,避免各页重复推断。
- `appVersion` 为可选展示信息。

```ts
// GET /app-info
interface AppInfo { runMode: RunMode; appVersion: string }  // RunMode = 'desktop' | 'server'(§4.3)
```

---

## 7. 一致性红线(设计与实现须守)

- **DTO 绝不暴露** `private_key_ref` / `account_key_ref` / 任何密钥材料;密钥仅经 §5 导出端点按引用读取。
- **枚举取 TECH §4.3 wire 值**,不自造;前端 import 自 bindings(L1)。
- **资源命名唯一**:`/certificates` `/domains` `/acme/accounts` `/acme/challenges` `/root-cas` `/tasks` `/settings` `/dashboard` `/app-info` `/events`。
- **动作 = POST 子资源**;各模块同构(签发 / 续签 / 吊销 / 重试 / 取消 / 导出)。
- **一个全局 SSE 流**([`events.md`](./events.md)),各模块引用、不各造一套。
- **时间 RFC3339 UTC / 分页 `{items,page,pageSize,total}` / 错误 `{error:{code,message,details}}`** 三协议定死不漂移(可沉淀为 protocolLints L2–L5)。
