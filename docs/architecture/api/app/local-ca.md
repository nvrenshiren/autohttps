# API 契约 · 自签根 CA(local-ca)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/local-ca.md §2 功能列表`(A1 查看、A2 创建、A3 导入、A4 导出;B1/B2 受托签发/作废=后端行为)· `flows/local-ca.md`(根 CA 状态机 2 态 L1–L3 · 无过渡态 LC1 · 多根并存 LC6 · 不显式移除 LC5 · 仅公开证书导出 LC4)· `database/local-ca.md` · 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:只管根 CA 本身;内网证书本体/状态/续签/到期/吊销归 certificates(走证书状态机,LC2);B1 签发 / B2 作废标记是 **certificates 委托的后端行为、非页面端点**。
> **不含**:内网证书列表/详情/管理(归 certificates)、批量操作、根 CA 自动续期(LC3)、含私钥/迁移导出(LC4)、显式移除根 CA(LC5)、自动安装信任库。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 | 映射转移 |
| --- | --- | --- | --- | --- |
| GET | `/root-cas` | 根 CA 列表 | A1 | — |
| GET | `/root-cas/{id}` | 根 CA 详情(含已签内网证书概览) | A1 | — |
| POST | `/root-cas` | 创建新根 CA(本地生成密钥对并自签) | A2 | L1 |
| POST | `/root-cas/import` | 导入已有根 CA(证书+配对私钥) | A3 | L2 |
| GET | `/root-cas/{id}/export` | 导出根 CA **公开证书**(二进制下载) | A4 | 跨状态动作 |

> **无 DELETE / 无 renew**:MVP 不显式移除(LC5)、无自动续期(LC3),过期后靠创建/导入新根接替。
> **B1 签发 / B2 作废标记无端点**:由执行器在 certificates 触发的 `self_signed` `issue`/`renew`/`revoke` 任务内完成(委托后端行为,modules §2-B 注);已过期根 CA 拒绝签发 → 该证书任务失败、证书转 `issue_failed`(证书状态机)。
> **导出=公开根 CA 证书**(LC4):不含私钥,故无私钥门控。

---

## 2. 端点详情

### 2.1 `GET /root-cas` — 列表(A1)

- **过滤**:`status`(`active`|`expired`)。排序 `notAfter`(默认 `asc`)/ `createdAt`。分页。
- 响应 `{ items: RootCaSummary[], … }`。

### 2.2 `GET /root-cas/{id}` — 详情(A1)

- 200 → `RootCaDetail`(含 `issuedCertificateCount`;内网证书明细经 `GET /certificates?rootCaId={id}` 于 certificates 侧查看);不存在 → `404 root_ca_not_found`。

### 2.3 `POST /root-cas` — 创建(A2,L1)

```ts
interface CreateRootCaRequest {
  name: string;
  validityDays: number;                 // 有效期(自 now);服务层算 notBefore/notAfter
  // 密钥算法等技术参数由 architect 定合理默认,不在业务契约暴露(modules/local-ca §3.1)
}
```
- **行为**:本地生成密钥对并自签(rcgen)→ 私钥密文落数据目录(`private_key_ref`)、公开证书内联 → 进入 `active`。**本地同步操作、无过渡态**(LC1)→ **201** + `RootCaDetail`。
- **校验**:`validityDays≤0` → `422 invalid_validity_period`;`name` 空 → `400 validation_failed`。

### 2.4 `POST /root-cas/import` — 导入(A3,L2)

```ts
interface ImportRootCaRequest {
  name: string;
  certPem: string;                       // 根 CA 证书(公开)
  privateKeyPem: string;                 // 配对私钥(敏感,校验后密文落地,不入库明文)
  keyPassphrase?: string;                // 私钥受口令保护时提供
}
```
- **行为**:校验证书↔私钥配对、证书为可用根 CA(x509-parser)→ 落地(私钥密文,证书内联)→ 进入 `active`;**若导入证书本身已过有效期则直接判 `expired`**(L2)。同步 → **201** + `RootCaDetail`。
- **校验**:证书/私钥不配对 → `422 import_key_mismatch`;非合法 CA 证书 → `422 import_invalid_certificate`;口令错/私钥无法解密 → `422 import_key_decryption_failed`。

### 2.5 `GET /root-cas/{id}/export` — 导出公开证书(A4)

- 导出**根 CA 证书 PEM**(公开部分),供客户端导入信任库。二进制下载(common §5):服务器=浏览器下载,桌面=Tauri 原生保存(同一端点、不同通道)。只读、不改状态。
- **MVP 不含私钥 / 不做迁移导出**(LC4);无 `parts`/`acknowledge` 参数。不存在 → `404 root_ca_not_found`。

---

## 3. 关键 DTO(camelCase,**不含** `private_key_ref`/私钥材料)

```ts
interface RootCaSummary {
  id: string;
  name: string;
  status: RootCaStatus;                 // §4.3:active | expired
  creationMethod: string;               // 'created' | 'imported'(局部属性,非 §4.3 枚举,DB §2 已标注治理路径)
  notBefore: string;
  notAfter: string;
  daysUntilExpiry: number;              // 计算字段:相对 now;已过期为负
  serialNumber: string | null;
  fingerprint: string | null;
  issuedCertificateCount: number;       // 该根 CA 签发的内网证书数(certificates.root_ca_id 反查)
  createdAt: string;
  updatedAt: string;
}
interface RootCaDetail extends RootCaSummary {
  certPem: string;                      // 根 CA 证书本体(公开材料,可内联返回;导出为下载形态)
}
```

> **密钥边界**:DTO **无** `privateKeyRef`/根 CA 私钥(敏感级最高,AR4);`certPem` 为公开材料可返回。**永不导出根 CA 私钥**(LC4)。`creationMethod` 沿 wire snake_case(`created`/`imported`),前端如需强类型消费须经 architect 纳入 §4.3(勿自造枚举)。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `root_ca_not_found` | 404 | 目标根 CA 不存在 |
| `invalid_validity_period` | 422 | 创建 `validityDays≤0` 或有效期非法 |
| `import_key_mismatch` | 422 | 导入证书与私钥不配对 |
| `import_invalid_certificate` | 422 | 导入证书非合法根 CA 证书 |
| `import_key_decryption_failed` | 422 | 私钥口令错 / 无法解密 |
| `root_ca_expired` | 422 | 用已过期根 CA 签发(**共享规则**,与 certificates 同义;实际在 self_signed 签发校验点触发) |

> 全局 `validation_failed`(400)/`internal_error`(500)见 common §4.3。

---

## 5. 状态机 → 端点映射(flows/local-ca §2.3)

| 转移 | 触发端点 / 来源 |
| --- | --- |
| L1 尚无→有效(创建) | `POST /root-cas` |
| L2 尚无→有效/已过期(导入) | `POST /root-cas/import` |
| L3 有效→已过期(扫描) | 扫描器(无端点;经 SSE `root_ca_status_changed`) |
| 签发内网证书(不改状态,受托) | 执行器内(certificates 的 self_signed 任务;无 local-ca 端点) |
| 标记内网证书作废(不改状态,受托) | 执行器内(certificates 的 revoke 任务;写 `internal_cert_revocations`) |
| 导出(不改状态) | `GET /root-cas/{id}/export` |

## 6. 本模块 SSE 事件(见 [`common/events.md`](./common/events.md))

- `root_ca_status_changed { rootCaId, status }` — 扫描判定 `active→expired`(L3)。前端失效根 CA 列表/详情;dashboard 可据此提示"链根到期影响其签出内网证书"(呈现口径归 dashboard)。
- 创建/导入为同步动作,前端 mutation 成功即失效,无需专门 SSE。

## 7. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。多根并存(LC6)、无移除(LC5)、仅公开证书导出(LC4)、无自动续期(LC3)均已裁决;B1/B2 委托后端行为不暴露为端点,与"非页面直达操作"一致。
