# API 契约 · 证书管理(certificates)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/certificates.md §2 功能列表`(A1/A2 查、B1/B2 签发、C1/C3 续签、D1/D2 吊销删除、E1 导出)· `flows/certificates.md`(证书状态机 10 态 · T1–T24)· `database/certificates.md`(实体→DTO)· 共用约定 [`common/conventions.md`](./common/conventions.md) · 全局事件 [`common/events.md`](./common/events.md)。
> **certificates 是全局枢纽**:证书↔域名(SAN)、证书↔任务、证书↔ACME 账户、证书↔根 CA 四类引用在此暴露为动作入参与 DTO。
> **不含**(遵 DEC2):批量签发/吊销/删除、统计卡片(归 dashboard)、自动部署(D3)。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 | 映射转移 |
| --- | --- | --- | --- | --- |
| GET | `/certificates` | 证书列表(筛选/分页) | A1 | — |
| GET | `/certificates/{id}` | 证书详情 | A2 | — |
| POST | `/certificates` | 发起首次签发(创建条目+入队签发任务) | B1 | T1 |
| POST | `/certificates/{id}/renew` | 续签 / 再获取(有效/即将到期/已过期/已吊销) | C1 | T7·T9·T17·T20 |
| POST | `/certificates/{id}/retry` | 失败重试(签发失败/续签失败) | B2·C3 | T5·T14 |
| POST | `/certificates/{id}/revoke` | 吊销(有效/即将到期/续签失败) | D1 | T8·T11·T16 |
| DELETE | `/certificates/{id}` | 删除证书条目+本地文件(非进行中态) | D2 | 退出状态机 |
| GET | `/certificates/{id}/export` | 导出叶子/链/私钥(二进制下载) | E1 | 跨状态动作 |

> **取消不在此模块**:进行中态(签发中/续签中/吊销中)的取消经 **tasks** 端点(`POST /tasks/{id}/cancel`)完成,驱动证书 T21–T24 回退——因 certificates §2 功能列表**无"取消"操作**,取消是 tasks §2-B2 能力(flows/certificates §2.4"经 tasks 取消";flows/tasks §4.4)。证书详情返回 `activeTaskId` 供前端定位该任务(见 §3.2)。
> **自动续签(C2)无端点**:由扫描器依 settings 自动触发(ARCHITECTURE §6.2 / T9·T14),非 operator 动作;经全局 SSE 回推状态。

---

## 2. 端点详情

### 2.1 `GET /certificates` — 列表(A1)

- **过滤**:`status`(证书状态枚举,可重复取多值)· `issuanceMethod`(`acme`|`self_signed`)· `domain`(hostname 子串,大小写不敏感,经 `certificate_domains`→`domains` 反查)。
- **排序**(`sort`):`notAfter`(默认,`order=asc` 便于"最快到期在前")· `createdAt` · `updatedAt`;默认 `sort=notAfter&order=asc`。
- **分页**:统一 `page`/`pageSize`(common §3)。
- **响应**:`{ items: CertificateSummary[], page, pageSize, total }`。

### 2.2 `GET /certificates/{id}` — 详情(A2)

- 200 → `CertificateDetail`;不存在 → `404 cert_not_found`。

### 2.3 `POST /certificates` — 发起首次签发(B1,T1)

- **请求体**:

```ts
interface IssueCertificateRequest {
  issuanceMethod: IssuanceMethod;   // 'acme' | 'self_signed'
  domainIds: string[];              // 已存在域名 id(≥1);SAN,至多一个通配符
  acmeAccountId?: string;           // 仅 acme:省略则用 settings 默认账户
  rootCaId?: string;                // 仅 self_signed:必填
}
```

- **行为**:校验通过 → 创建证书条目(`pending_issue`,T1)+ 入队 `issue` 任务 → **202** + `CertificateDetail`(状态 `pending_issue`);后续 `issuing/valid/issue_failed` 经 SSE 回推。
- **校验(422 除非注明)**:
  - `domainIds` 空 → `no_domains_specified`;含不存在域名 → `invalid_domain_reference`(`details.domainIds`)。
  - 含通配符域名但其 `validationMethod≠dns_01` → `wildcard_requires_dns01`(共享规则)。
  - SAN 内 >1 个通配符 → `multiple_wildcards_not_allowed`(DEC4/§3.3)。
  - `acme`:`acmeAccountId` 省略且无 settings 默认账户 → `acme_account_required`;指定账户不存在 → `invalid_acme_account_reference`;账户非 `registered` → `acme_account_not_registered`(共享规则);任一域名 `validationMethod` 未设置 → `domain_validation_method_required`(acme 需按域名验证)。
  - `self_signed`:`rootCaId` 缺失 → `root_ca_required`;不存在 → `invalid_root_ca_reference`;根 CA 非 `active` → `root_ca_expired`(共享规则)。
  - 同时给 `acmeAccountId` 与 `rootCaId`,或与 `issuanceMethod` 不符 → `issuance_source_conflict`(枢纽 XOR 不变量,_overview §4.1)。

### 2.4 `POST /certificates/{id}/renew` — 续签 / 再获取(C1)

- **适用源态 → 转移**:`valid`(T7 提前续)· `expiring_soon`(T9)· `expired`(T17)· `revoked`(T20 换新私钥重新签发)。→ `renewing`,**202**。
- 非适用态(如 `issuing`/`renewing`/`revoking`/`pending_issue`/`issue_failed`/`renewal_failed`)→ `409 invalid_cert_state`(`details.currentStatus`)。`renewal_failed` 的再续用 `retry`(见 §2.5)。
- self_signed 证书续签同样校验其 `root_ca_id` 仍 `active`,否则 `422 root_ca_expired`;acme 证书校验账户仍 `registered`,否则 `422 acme_account_not_registered`(账户被移除置空则 `acme_account_required`,提示改选)。

### 2.5 `POST /certificates/{id}/retry` — 失败重试(B2·C3)

- **适用源态 → 转移**:`issue_failed`(T5 → `issuing`)· `renewal_failed`(T14 → `renewing`)。派生新任务(tasks TT7),**202**。
- 非失败态 → `409 invalid_cert_state`。
- 续签失败重试(C3)同 renew 的来源校验(账户/根 CA 前置)。

### 2.6 `POST /certificates/{id}/revoke` — 吊销(D1)

- **适用源态 → 转移**:`valid`(T8)· `expiring_soon`(T11)· `renewal_failed`(T16,旧证书尚在有效期内)。→ `revoking`,**202**;成功 T18→`revoked`,失败 T19 回退原态(经 SSE)。
- 非适用态 → `409 invalid_cert_state`。
- acme 证书向 CA 发吊销;self_signed 由 local-ca 根 CA 记作废(委托,执行器内完成)。

> **⚠ PRD 口径提示(见 §5 缺口)**:flows/certificates §3.5 正文列 `expired` 为可吊销,但 §2.3 权威转移表(T8/T11/T16)**无** `expired→revoking`。本契约以**权威转移表**为准(revoke 不含 `expired`)。

### 2.7 `DELETE /certificates/{id}` — 删除(D2)

- **适用**:非进行中态(`pending_issue`/`issue_failed`/`valid`/`expiring_soon`/`renewal_failed`/`expired`/`revoked`)。
- 进行中态(`issuing`/`renewing`/`revoking`)→ `409 cert_in_progress_cannot_delete`(须先经 `POST /tasks/{id}/cancel` 取消其任务)。
- **行为**:移除证书条目 + 本地证书/私钥文件(按 `*_ref` 清除密文);其**未完成任务**由 tasks 一并取消(`trigger=cleanup`,tasks §5.5);**历史任务只读保留**、软引用不级联(DT3/Q2)。**204**。
- 二次确认为 UI 职责,端点不设 `confirm` 参数。
- 成功后 SSE `certificate_status_changed`(该证书退出)+ 关联 `task_status_changed`(清理取消)+ `dashboard_changed`。

### 2.8 `GET /certificates/{id}/export` — 导出(E1)

- **查询**:`parts=<逗号分隔:leaf|chain|fullchain|private_key>`(默认 `fullchain`)· `format=pem`(MVP 仅 PEM)· 含 `private_key` 时须 `acknowledgeKeyExport=true`。
- **响应**:二进制 PEM 下载(common §5);服务器=浏览器下载,桌面=Tauri 原生保存(同一端点、不同通道)。
- **错误**:无文件态导出 → `409 cert_not_exportable`;含私钥未确认 → `422 key_export_not_acknowledged`;`parts` 非法值 → `400 validation_failed`。

---

## 3. 关键 DTO(camelCase,**不含** `private_key_ref`/`cert_pem_ref`/密钥材料)

### 3.1 `CertificateSummary`(列表项)

```ts
interface CertificateSummary {
  id: string;
  status: CertificateStatus;          // §4.3 证书状态 10 态
  issuanceMethod: IssuanceMethod;     // 'acme' | 'self_signed'
  domains: DomainRef[];               // SAN 域名(仅标识,不含域名全量)
  serialNumber: string | null;        // 未签发前 null
  notBefore: string | null;           // RFC3339
  notAfter: string | null;            // RFC3339
  daysUntilExpiry: number | null;     // 计算字段:相对服务器 now;已过期为负;未签发 null
  isExportable: boolean;              // 本地是否已有文件(status 有文件态)
  lastError: string | null;           // 最近失败原因摘要(展示用;完整日志在 tasks)
  updatedAt: string;
}
interface DomainRef { id: string; hostname: string; isWildcard: boolean }
```

### 3.2 `CertificateDetail`(详情)

```ts
interface CertificateDetail extends CertificateSummary {
  fingerprint: string | null;
  issuedAt: string | null;                       // 最近一次成功签发/续签落地
  createdAt: string;
  acmeAccount: AcmeAccountRef | null;            // 仅 acme;账户被移除置 null(需改选)
  rootCa: RootCaRef | null;                      // 仅 self_signed
  activeTaskId: string | null;                   // 当前进行中任务(供进行中态经 tasks 取消)
}
interface AcmeAccountRef { id: string; caLabel: string | null; environment: string | null }
interface RootCaRef { id: string; name: string }
```

> **密钥边界**:DTO **无** `privateKeyRef`/`certPemRef`/任何密钥字段;证书链/私钥仅经 §2.8 导出端点按引用读取(AR4)。`acmeAccount`/`rootCa` 仅回投影引用,不泄露账户密钥/根 CA 私钥。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `cert_not_found` | 404 | 目标证书不存在 |
| `cert_in_progress_cannot_delete` | 409 | 对 `issuing`/`renewing`/`revoking` 删除(§2.7) |
| `cert_not_exportable` | 409 | 对无文件态(`pending_issue`/`issuing`/`issue_failed`)导出(§2.8) |
| `invalid_cert_state` | 409 | renew/retry/revoke 用于不适用源态;`details.{currentStatus,action}` |
| `no_domains_specified` | 422 | 签发请求 `domainIds` 为空 |
| `invalid_domain_reference` | 422 | 引用不存在域名;`details.domainIds` |
| `multiple_wildcards_not_allowed` | 422 | SAN 含 >1 通配符(DEC4/§3.3) |
| `domain_validation_method_required` | 422 | acme 签发但某域名未设 `validationMethod` |
| `wildcard_requires_dns01` | 422 | 通配符域名验证方式非 `dns_01`(**共享规则**,与 domains 同义) |
| `issuance_source_conflict` | 422 | 账户/根 CA 与方式不符或同时给出(枢纽 XOR) |
| `acme_account_required` | 422 | acme 方式无账户且无默认账户 |
| `invalid_acme_account_reference` | 422 | 引用不存在 ACME 账户 |
| `acme_account_not_registered` | 422 | 指定账户非 `registered`(**共享规则**,与 acme 同义) |
| `root_ca_required` | 422 | self_signed 缺 `rootCaId` |
| `invalid_root_ca_reference` | 422 | 引用不存在根 CA |
| `root_ca_expired` | 422 | 指定根 CA 非 `active`(**共享规则**,与 local-ca 同义) |
| `key_export_not_acknowledged` | 422 | 导出含私钥未带 `acknowledgeKeyExport=true` |

> 全局 `validation_failed`(400)/`internal_error`(500)见 common §4.3。异步执行失败(域名验证失败/CA 拒绝/自签失败)不经此表返回,而以任务 `failureReason` + 日志承载,证书转 `issue_failed`/`renewal_failed` 经 SSE 回推。

---

## 5. 状态机 → 端点映射(权威:flows/certificates §2.3)

| 转移 | 触发端点 / 来源 |
| --- | --- |
| T1 待签发 ← 发起首签 | `POST /certificates` |
| T2/T3/T4 签发中/有效/签发失败 | 任务执行器(无端点;经 SSE) |
| T5 签发失败→签发中 | `POST /certificates/{id}/retry` |
| T6/T10 扫描到期 | 扫描器(无端点) |
| T7 有效→续签中 · T9 即将到期→续签中 · T17 已过期→续签中 · T20 已吊销→续签中 | `POST /certificates/{id}/renew` |
| T8 有效→吊销中 · T11 即将到期→吊销中 · T16 续签失败→吊销中 | `POST /certificates/{id}/revoke` |
| T12/T13 续签成功/失败 · T18/T19 吊销成功/失败回退 | 执行器(无端点;经 SSE) |
| T14 续签失败→续签中 | `POST /certificates/{id}/retry` |
| T21/T22 取消首签→签发失败 · T23 取消续签回退 · T24 取消吊销回退 | `POST /tasks/{id}/cancel`(tasks 端点驱动) |
| 删除(退出状态机) | `DELETE /certificates/{id}` |
| 导出(跨状态动作) | `GET /certificates/{id}/export` |

## 6. 本模块 SSE 事件(见 [`common/events.md`](./common/events.md))

- 发出:`certificate_status_changed { certificateId, status }`——任一证书状态流转(执行器结果/扫描/取消回退)。
- 引发:`dashboard_changed`(证书进/出待处理集时,由聚合层合并发出)。

---

## 7. PRD/DB 缺口(architect 停止条件核查)

- **非阻塞·口径提示**:flows/certificates §3.5 正文含 `expired` 可吊销,但 §2.3 权威转移表无 `expired→revoking`(T8/T11/T16 仅 valid/expiring_soon/renewal_failed)。本契约按**权威转移表**设计 revoke 适用态、**未擅自新增** `expired→revoking`。建议 PM 校订 §3.5 正文或补 T 转移;不阻塞本契约交付。其余功能/数据来源明确,无阻塞缺口。
