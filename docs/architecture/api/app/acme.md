# API 契约 · ACME 签发(acme)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/acme.md §2 功能列表`(A1–A5 账户、B1–B6 验证向导与挑战)· `flows/acme.md`(账户状态机 4 态 AT1–AT5 · 挑战状态机 6 态 CT1–CT10 · §3.4 多域名整体判定)· `database/acme.md` · 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:被 certificates 委托验证+取证,取证后交回、**不留存证书本体**(DEA4);验证方式**类别**关联归 domains,本模块持 **webroot 执行配置** + 挑战记录(DEA5);默认账户归 settings(仅消费)。
> **不含**:DNS 厂商 API 自动验证(仅手动 DNS-01,DEA2)、证书存储/到期/吊销对外动作(归 certificates)、批量账户操作、统计卡片。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 | 映射转移 |
| --- | --- | --- | --- | --- |
| GET | `/acme/accounts` | ACME 账户列表 | A2 | — |
| GET | `/acme/accounts/{id}` | 账户详情 | A2 | — |
| POST | `/acme/accounts` | 配置并注册账户 | A1 | AT1 |
| PATCH | `/acme/accounts/{id}` | 编辑联系邮箱(已注册) | A3 | 跨状态动作 |
| POST | `/acme/accounts/{id}/retry` | 注册失败重试 | A4 | AT4 |
| DELETE | `/acme/accounts/{id}` | 移除账户(带影响提示) | A5 | AT5 |
| GET | `/acme/http01-configs/{domainId}` | 查看某域名 HTTP-01 webroot 配置 | B2 | — |
| PUT | `/acme/http01-configs/{domainId}` | 设置/更新 webroot 路径 | B2 | — |
| GET | `/acme/challenges` | 挑战列表(按任务/域名/状态筛选) | B6 | — |
| GET | `/acme/challenges/{id}` | 挑战详情(TXT/HTTP 文件、状态) | B4 · B6 | — |
| GET | `/acme/challenges/{id}/dns-precheck` | DNS-01 提交前本地预检 TXT 是否生效 | B4(可选) | — |
| POST | `/acme/challenges/{id}/confirm` | DNS-01 确认已添加 TXT,触发校验 | B4 | CT4 |
| POST | `/acme/challenges/{id}/retry` | 挑战失败重试 | B5 | CT7 |

> **验证方式向导(B1)= 前端编排**:调 domains `PATCH validationMethod`(类别落 domains)+ 本模块 `PUT /acme/http01-configs/{domainId}`(HTTP-01 webroot);无独立"向导"端点。
> **执行验证挑战(B3)= 被 certificates 委托**:挑战由执行器在 `issue`/`renew` 任务运行时创建并推进(委托后端行为),**非页面直达端点**;operator 仅对 DNS-01 手动确认(confirm)、失败重试(retry)、查看(GET)。
> **放弃挑战(CT9 使用者放弃)/ 上游取消(CT9·CT10)= 经 tasks 取消**:`POST /tasks/{id}/cancel` 取消上游签发/续签任务,联动挑战转 `cancelled`(acme §2 无独立"放弃挑战"操作)。

---

## 2. 端点详情

### 2.1 账户

**`GET /acme/accounts`(A2)** — 过滤 `status`;排序 `createdAt`(默认 desc);分页。响应 `{ items: AcmeAccountSummary[], … }`。

**`POST /acme/accounts` — 配置并注册(A1,AT1)**

```ts
interface RegisterAcmeAccountRequest {
  directoryUrl: string;    // ACME 目录端点 URL——唯一标定"目标 CA + 环境"
  caLabel?: string;        // CA 展示名(如 "Let's Encrypt")
  contactEmail: string;
  tosAgreed: boolean;      // 须为 true(AT1 前提)
}
```
- **行为**:创建账户(`registering`)+ 生成账户密钥(密文落数据目录,`account_key_ref`)+ 向 CA 注册 → **202** + `AcmeAccountDetail`;终态 `registered`/`registration_failed` 经 SSE `acme_account_status_changed` 回推。
- **校验**:`tosAgreed≠true` → `422 tos_not_agreed`;`directoryUrl` 非法/空 → `422 invalid_directory_url`;`contactEmail` 格式错 → `400 validation_failed`。
- 注:注册**非** tasks 任务(tasks 只 issue/renew/revoke,DEC5);由 acme 服务异步执行。允许同 CA 同邮箱多账户(DA3,不设唯一约束)。

**`PATCH /acme/accounts/{id}` — 编辑邮箱(A3)** — 体 `{ contactEmail }`;仅 `registered` 态可编辑,否则 `409 account_state_invalid`;必要时对 CA 更新账户信息(业务仍同一账户)。→ 200。

**`POST /acme/accounts/{id}/retry` — 注册失败重试(A4,AT4)** — 仅 `registration_failed` → `registering`(**202**);其他态 → `409 account_state_invalid`。可先 PATCH 修正邮箱再重试。

**`DELETE /acme/accounts/{id}` — 移除(A5,AT5)** — 退出账户状态机、清除账户密钥材料;引用该账户的证书 `acme_account_id` 置空(SET NULL,续签需改选账户)、settings 默认账户指向置空(SET NULL)。→ **204**。**影响提示**:删除前 UI 依 `certificateCount`(账户 DTO)展示"N 张证书正引用"并二次确认;端点本身直接执行(不 RESTRICT)。不存在 → `404 acme_account_not_found`。

### 2.2 HTTP-01 webroot 配置(B2,DEA5)

- **`PUT /acme/http01-configs/{domainId}`** — 体 `{ webrootPath }`;按域名 upsert(1:0..1)。域名不存在 → `404 domain_not_found`;`webrootPath` 空 → `400 validation_failed`。→ 200/201 `Http01Config`。
- **`GET /acme/http01-configs/{domainId}`** — 未配置 → `404 http01_config_not_found`。
- 注:DNS-01 无常驻配置(挑战时动态生成 TXT,DS2),故本资源仅 HTTP-01。

### 2.3 挑战(B4/B5/B6)

**`GET /acme/challenges`(B6)** — 过滤 `taskId` · `domainId` · `status`(含 `awaiting_manual`,dashboard 待处理源)· `certificateId`(经 task 关联反查);排序 `createdAt`(默认 desc);分页。

**`GET /acme/challenges/{id}`(B4·B6)** — `ChallengeDetail`(DNS-01 展示 TXT 名/值供复制;HTTP-01 展示文件路径/内容)。

**`GET /acme/challenges/{id}/dns-precheck`(B4 可选)** — 本地 `hickory-resolver` 查询该挑战 TXT 是否已生效 → `{ propagated: boolean, observedValues: string[] }`;只读、不改挑战状态。仅 DNS-01 挑战适用,否则 `422 not_dns01_challenge`。

**`POST /acme/challenges/{id}/confirm`(B4,CT4)** — DNS-01 确认已添加 TXT → 请求 CA 校验,挑战 `awaiting_manual→validating`(**202**);终态 `passed`/`failed` 经 SSE。仅 `awaiting_manual` 态可确认,否则 `409 challenge_not_awaiting_manual`。

**`POST /acme/challenges/{id}/retry`(B5,CT7)** — 仅 `failed` 态 → 重新发起验证(必要时重建订单取新挑战:HTTP-01 重放文件 / DNS-01 重新展示 TXT),挑战 `failed→pending`(**202**);非失败态 → `409 challenge_not_retryable`。

> **多域名整体判定(§3.4)**:一次委托含多域名 SAN、每域名一挑战;全部 `passed` 方可取证,任一 `failed`/`cancelled` 则整体失败(交回 certificates 转 `issue_failed`/`renewal_failed`)。此判定在执行器内,契约层经 `challenges?taskId=` 可观察各域名挑战态。

---

## 3. 关键 DTO(camelCase,**不含** `account_key_ref`/密钥材料)

```ts
interface AcmeAccountSummary {
  id: string;
  directoryUrl: string;
  caLabel: string | null;
  environment: string | null;         // 生产/测试 展示标签(非 §4.3 枚举,DB §2 已标注)
  contactEmail: string;
  status: AcmeAccountStatus;           // §4.3:registering|registered|registration_failed(unconfigured 为概念态,持久行不取)
  isDefault: boolean;                  // 计算:settings.defaultAcmeAccountId 是否指向此账户
  certificateCount: number;            // 影响提示:多少证书引用此账户(A5 移除确认)
  registeredAt: string | null;
  lastError: string | null;           // registration_failed 时展示
  createdAt: string;
  updatedAt: string;
}
interface AcmeAccountDetail extends AcmeAccountSummary {
  caAccountUrl: string | null;        // CA 返回的账户资源 URL(account kid)
  tosAgreed: boolean;
}

interface Http01Config { domainId: string; webrootPath: string; updatedAt: string }

interface ChallengeSummary {
  id: string;
  taskId: string;
  certificateId: string;               // 经 task 关联
  domainId: string;
  domainHostname: string | null;       // 解析展示(域名可能已删 → null)
  validationMethod: ValidationMethod;  // http_01 | dns_01
  status: ChallengeStatus;             // §4.3 挑战状态 6 态
  failedReason: string | null;         // 失败原因摘要(如 challenge_timeout;完整日志在 tasks)
  createdAt: string;
  updatedAt: string;
}
interface ChallengeDetail extends ChallengeSummary {
  dnsTxtName: string | null;           // DNS-01:待添加 TXT 记录名(展示供复制)
  dnsTxtValue: string | null;          // DNS-01:待添加 TXT 记录值(非密钥,可展示)
  httpFilePath: string | null;         // HTTP-01:验证文件路径
  httpFileContent: string | null;      // HTTP-01:key authorization(非私钥,可展示)
}
```

> **密钥边界**:账户 DTO **无** `accountKeyRef`/账户密钥;真正敏感的账户密钥仅 core secrets 持有(AR4)。挑战的 TXT 值 / HTTP 文件内容 / key authorization **非** AR4 敏感三类,可展示(DNS-01 需供复制,acme DB §4.2)。acme **不返回证书本体**(DEA4)。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `acme_account_not_found` | 404 | 目标账户不存在 |
| `challenge_not_found` | 404 | 目标挑战不存在 |
| `http01_config_not_found` | 404 | 该域名无 HTTP-01 webroot 配置 |
| `account_state_invalid` | 409 | 编辑非 `registered` 账户 / 重试非 `registration_failed` 账户 |
| `challenge_not_awaiting_manual` | 409 | confirm 用于非 `awaiting_manual` 挑战 |
| `challenge_not_retryable` | 409 | retry 用于非 `failed` 挑战 |
| `tos_not_agreed` | 422 | 注册未同意服务条款(AT1) |
| `invalid_directory_url` | 422 | ACME 目录 URL 非法 |
| `acme_account_not_registered` | 422 | 用未注册账户执行(**共享规则**,与 certificates 同义) |
| `not_dns01_challenge` | 422 | 对非 DNS-01 挑战 dns-precheck |
| `domain_not_found` | 404 | webroot 配置引用不存在域名(**共享**,与 domains 同义) |

> **异步失败原因**(如 `challenge_timeout` CT8、TXT 不符、CA 拒绝、网络错误):不作为 HTTP 请求错误返回,而写入 `challenge.failedReason` 摘要 + `task_log_entries` 日志(脱敏),挑战转 `failed` 经 SSE 回推。`failedReason` 为人读摘要,非 §4.3 枚举(不自造)。

---

## 5. 状态机 → 端点映射

**ACME 账户(flows/acme §2.3):** AT1 注册 ← `POST /acme/accounts`;AT2/AT3 注册成功/失败 ← acme 服务异步(无端点,经 SSE);AT4 重试 ← `POST /acme/accounts/{id}/retry`;AT5 移除 ← `DELETE /acme/accounts/{id}`;编辑邮箱(跨状态)← `PATCH /acme/accounts/{id}`。

**验证挑战(flows/acme §3.3):** CT1 建立挑战/CT2 HTTP-01 自动校验/CT5/CT6/CT8 ← 执行器(无端点,经 SSE);CT3 待验证→等待手动配置 ← 执行器发 `challenge_status_changed(awaiting_manual)`;CT4 确认→验证中 ← `POST /acme/challenges/{id}/confirm`;CT7 失败→重试 ← `POST /acme/challenges/{id}/retry`;CT9/CT10 取消 ← `POST /tasks/{id}/cancel`(tasks 驱动)。

## 6. 本模块 SSE 事件(见 [`common/events.md`](./common/events.md))

- `acme_account_status_changed { accountId, status }` — 注册完成/失败(AT2/AT3)。
- `challenge_status_changed { challengeId, taskId, domainId, status }` — 挑战流转;**`status=awaiting_manual` 是 DNS-01 待处理提示 + 红点来源**(CT3)。
- 引发 `dashboard_changed`(挑战进入/离开 `awaiting_manual`,由聚合层合并发出)。

## 7. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。账户/挑战状态机、多账户(DA3)、手动 DNS-01(DA2)、webroot 归属(DEA5)均已裁决且明确。account registration 非 task 的处置与状态机 `registering` 过渡态一致。
