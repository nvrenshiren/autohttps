# API 契约 · 域名管理(domains)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/domains.md §2 功能列表`(A1/A2 查、B1/B2/B3 维护、C1/C2 关联)· `flows/domains.md`(域名无独立状态机 DECD1 · 删除硬拦截 DECD3 · hostname 不可改 DECD2)· `database/domains.md` · 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:证书生命周期归 certificates;证书态为**投影**(只读);验证方式**类别**关联归本模块,webroot 执行配置归 acme(DEA5)。
> **不含**:批量新增/删除、统计卡片(归 dashboard)、证书生命周期动作(归 certificates)、验证方式配置与执行(归 acme)、域名启停(无状态机)。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 | 功能 |
| --- | --- | --- | --- |
| GET | `/domains` | 域名列表(筛选/分页,含证书态投影) | A1 |
| GET | `/domains/{id}` | 域名详情(含关联证书清单投影) | A2 · C1 |
| POST | `/domains` | 新增域名 | B1 |
| PATCH | `/domains/{id}` | 编辑分组/备注/验证方式关联(hostname 不可改) | B2 · C2 |
| DELETE | `/domains/{id}` | 删除域名(被证书关联则拦截) | B3 |

> **验证方式关联(C2)= PATCH `validationMethod` 字段**,不单列端点。**签发入口(C1)= 前端跳转 certificates 签发页**(预置本域名),关联建立在 certificates 签发流程,domains 无签发端点。**webroot 配置**归 acme(`PUT /acme/http01-configs/{domainId}`,见 acme 契约)。

---

## 2. 端点详情

### 2.1 `GET /domains` — 列表(A1)

- **过滤**:`group`(分组精确)· `certificateState`(证书态投影,见 §3 说明)· `hostname`(子串,大小写不敏感)。
- **排序**(`sort`):`hostname`(默认 `asc`)· `createdAt` · `updatedAt`。
- **响应**:`{ items: DomainSummary[], page, pageSize, total }`。

### 2.2 `GET /domains/{id}` — 详情(A2 · C1)

- 200 → `DomainDetail`(含 `certificates` 投影清单,可跳转证书详情);不存在 → `404 domain_not_found`。

### 2.3 `POST /domains` — 新增(B1)

```ts
interface CreateDomainRequest {
  hostname: string;                    // 身份;服务层校验格式,据 '*.' 前缀判定 isWildcard
  groupName?: string;
  remark?: string;
  validationMethod?: ValidationMethod; // 'http_01' | 'dns_01';通配符须 dns_01
}
```

- **行为**:校验格式 + 同实例唯一 → 落库 → **201** + `DomainDetail`(证书态 `none`)。
- **校验**:格式非法 → `400 validation_failed`;同实例 hostname 已存在 → `409 domain_already_exists`;通配符但 `validationMethod=http_01` → `422 wildcard_requires_dns01`。

### 2.4 `PATCH /domains/{id}` — 编辑(B2 · C2)

```ts
interface UpdateDomainRequest {                 // 均可选;仅改传入字段
  groupName?: string | null;                    // null 清除分组
  remark?: string | null;
  validationMethod?: ValidationMethod | null;   // 设置/清除 验证方式类别关联(C2)
}
```

- **行为**:更新可变字段 → **200** + `DomainDetail`。
- **hostname 不可改**(DECD2):请求体含 `hostname` → `422 hostname_immutable`(改名 = 删 + 增)。
- **通配符约束**:对 `isWildcard=true` 域名设 `validationMethod=http_01` → `422 wildcard_requires_dns01`(**共享规则**)。

### 2.5 `DELETE /domains/{id}` — 删除(B3)

- **前置硬拦截**(DECD3):被任一现存证书关联(`certificate_domains` 有行)→ `409 domain_has_certificates`(`details.certificateCount`);须先在 certificates 处理相关证书(不对已吊销/已过期放宽)。
- 无证书关联 → 移除域名对象及元数据 → **204**。
- 不存在 → `404 domain_not_found`。

---

## 3. 关键 DTO(camelCase)

```ts
interface DomainSummary {
  id: string;
  hostname: string;
  isWildcard: boolean;                          // 由 hostname 判定
  groupName: string | null;
  remark: string | null;
  validationMethod: ValidationMethod | null;    // 类别关联(webroot 在 acme)
  certificateCount: number;                     // 被多少现存证书关联
  worstCertificateStatus: CertificateStatus | null; // 证书态投影:null=无证书;否则取"最紧急"关联证书态
  updatedAt: string;
}
interface DomainDetail extends DomainSummary {
  createdAt: string;
  certificates: DomainCertificateRef[];         // 关联证书清单投影(C1;可跳转证书详情)
}
interface DomainCertificateRef {
  id: string;
  status: CertificateStatus;                     // 投影自 certificates,不复述定义
  issuanceMethod: IssuanceMethod;
  notAfter: string | null;
  daysUntilExpiry: number | null;
}
```

> **证书态投影(DS3)**:`worstCertificateStatus`/`certificates` 均**只读派生自 certificates**(经 `certificate_domains` 反查),domains 不落证书状态副本、不定义证书状态(flows/domains §2.1)。列表过滤 `certificateState` 的可选值与"最紧急"聚合口径由页面 PRD 细化;契约层以 `CertificateStatus` 枚举 + `none`(无证书)表达,不新增枚举。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `domain_not_found` | 404 | 目标域名不存在 |
| `domain_already_exists` | 409 | 同实例 hostname 重复(唯一约束) |
| `domain_has_certificates` | 409 | 删除被证书关联的域名(DECD3);`details.certificateCount` |
| `hostname_immutable` | 422 | PATCH 试图改 hostname(DECD2) |
| `wildcard_requires_dns01` | 422 | 通配符域名验证方式设为非 `dns_01`(**共享规则**,与 certificates 同义) |

> 格式非法(hostname 不合法、字段类型错)→ 全局 `validation_failed`(400)。

---

## 5. 状态机与 SSE

- **无独立状态机**(DECD1):域名对象一经创建即"存在",无生命周期流转,故无"状态机→端点"映射。
- **本模块不发 SSE 事件**:域名增删改为 operator 同步动作,前端 react-query 于 mutation 成功即本地失效;**证书态投影的新鲜度**由 `certificate_status_changed` 驱动(前端收到后一并失效域名列表/详情的投影)。见 [`common/events.md`](./common/events.md)。

## 6. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。功能列表、数据来源(DS1–DS4)、边界(DEA5 已裁决)明确;webroot 归 acme、类别关联归 domains 的切分清晰。
