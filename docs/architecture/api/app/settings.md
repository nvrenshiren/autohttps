# API 契约 · 系统设置(settings)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/settings.md §2 功能列表 / §4 数据来源`(运行形态配置 / 存储路径 / 续签策略 / 默认 ACME 账户)· `flows/settings.md`(无状态机 SF1 · 存储路径只读 SF5 · 无重试参数 SF2 · 不切换形态 SF4)· `database/settings.md`(单例表)· 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:ACME 账户本体归 acme(本模块仅存"默认指向");运行形态为运行时探测(经 `GET /app-info`,common §6.2),本模块不落形态、不提供切换(SF4)。
> **不含**:登录 / 权限 / 账户设置(D4 无鉴权)· 通知渠道配置(project §6.2)· 主题 / 语言 / 日志级别 · 续签重试次数 / 间隔(SF2)· 扫描周期(SF3)· 形态切换 / 数据迁移(SF4)。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| GET | `/settings` | 读取全局配置(单例) |
| PATCH | `/settings` | 修改可变配置项(存储路径只读;按当前运行形态适用) |

> 运行形态标志(`runMode`)与应用版本经 `GET /app-info`(common §6.2)取得,**不在本资源**;前端据 `runMode` 决定 autostart(仅桌面)vs 监听地址 / 端口(仅服务器)的显隐。**无 POST / DELETE**:settings 是长存单例,不创建、不删除。

---

## 2. 端点详情

### 2.1 `GET /settings` — 读取(查看)

- 200 → `SettingsView`(单例全字段);单例恒存(首次读取即 upsert 默认行,database §2.2),无 `404`。
- `dataStoragePath` 为**只读展示**(SF5);与当前形态不适用的字段(如服务器形态下的 `autostartEnabled`)返回 `null`。

### 2.2 `PATCH /settings` — 修改(配置)

```ts
interface UpdateSettingsRequest {          // 均可选;仅改传入字段
  renewalAdvanceDays?: number;             // 正整数(≥1);certificates 据此判"即将到期"(T6)
  autoRenewEnabled?: boolean;              // 自动续签开关
  defaultAcmeAccountId?: string | null;    // 默认 ACME 账户指向;null 清除
  autostartEnabled?: boolean;              // 仅桌面
  listenAddress?: string;                  // 仅服务器
  listenPort?: number;                     // 仅服务器(合法端口范围)
}
```

- **行为**:更新传入的可变字段 → **200** + `SettingsView`。settings 无状态机(SF1),即时覆盖生效。
- **只读拦截**:请求体含 `dataStoragePath` → `422 storage_path_read_only`(SF5:运行期不可改、无迁移)。
- **形态适用校验**:改与当前 `runMode` 不适用的字段(服务器形态改 `autostartEnabled`,或桌面形态改 `listenAddress` / `listenPort`)→ `422 setting_not_applicable`(`details.field` / `details.runMode`)。
- **默认账户校验**:`defaultAcmeAccountId` 指向不存在的账户 → `422 acme_account_not_found`(`details.id`);账户本体在 acme,本模块只存指向(database 纪律)。
- **入参校验**:`renewalAdvanceDays` 非正整数 / `listenPort` 越界 → `400 validation_failed`。
- **生效时机**:`renewalAdvanceDays` / `autoRenewEnabled` / `defaultAcmeAccountId` 对后续扫描与签发即时生效;`listenAddress` / `listenPort` / `autostartEnabled` 涉运行载体,**生效时机**(即时 / 需重启守护进程 / 需重登录桌面会话)属实现(flows/settings §2 生效口径),契约层标注、不承诺机制。

---

## 3. 关键 DTO(camelCase)

```ts
interface SettingsView {
  renewalAdvanceDays: number;              // 续签提前天数
  autoRenewEnabled: boolean;               // 自动续签开关
  defaultAcmeAccountId: string | null;     // 默认 ACME 账户指向(账户明细在 acme)
  autostartEnabled: boolean | null;        // 仅桌面;服务器形态为 null
  listenAddress: string | null;            // 仅服务器;桌面形态为 null
  listenPort: number | null;               // 仅服务器;桌面形态为 null
  dataStoragePath: string;                 // 只读展示(SF5)
  updatedAt: string;                       // RFC3339
}
```

> 形态相关字段按当前 `runMode` 取其一有值、另一组 `null`(database 形态差异);前端依 `runMode`(`GET /app-info`)显隐,**不据字段是否 `null` 反推形态**。`dataStoragePath` 仅供展示,PATCH 不接受(只读)。DTO 不含任何密钥 / `*_ref`(数据存储路径本身非密钥;私钥落该路径下但由各模块 `*_ref` 引用,不在 settings)。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `storage_path_read_only` | 422 | PATCH 试图改 `dataStoragePath`(SF5) |
| `setting_not_applicable` | 422 | 改与当前运行形态不适用的字段(仅桌面 / 仅服务器项);`details.field,runMode` |
| `acme_account_not_found` | 422 | `defaultAcmeAccountId` 指向不存在账户;`details.id`(**共享规则**,与 acme 同义、单一语义) |

> 结构 / 类型非法(`renewalAdvanceDays` 非正整数、`listenPort` 越界、字段类型错)→ 全局 `validation_failed`(400)。

---

## 5. 状态机与 SSE

- **无独立状态机**(SF1):配置项无生命周期流转,故无"状态机 → 端点"映射。
- **本模块不发 SSE 事件**:设置修改为 operator 同步动作,前端 react-query 于 mutation 成功即失效 `/settings`。续签策略变更的**后续效应**(某证书据新提前天数在下次扫描转 `expiring_soon`)经 `certificate_status_changed` 驱动(common/events.md),非 settings 直接推送。

---

## 6. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。功能列表(查看 / 修改各配置)、数据来源(DS1–DS5)、只读 / 形态差异 / 无重试参数(SF2–SF5)均明确;运行形态经 `/app-info` 取得(common),职责边界清晰。
