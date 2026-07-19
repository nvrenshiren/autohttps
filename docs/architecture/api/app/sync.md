# API 契约 · 备份同步(sync / WebDAV)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术契约(API)· 端点: app · 撰写: architect
> 依据(approved,唯一设计依据): `modules/sync.md §2 功能列表 / §4 数据来源 / §5 决策记录`(手动快照 DEC1 · 整目录快照 DEC2 · 整包加密 DEC3 · 口令分流 DEC4 · 覆盖式恢复 DEC5 · 独立远程目录 DEC6)· `database/sync.md`(sync_configs 单例)· 共用约定 [`common/conventions.md`](./common/conventions.md)。
> **边界**:本模块只做「整目录快照 ↔ WebDAV」的传输与落法;快照**内容**(各业务表 / 密钥材料)的语义归各业务模块。WebDAV 登录口令经统一 SecretStore 存引用(project §7),库内只存 `password_ref`,**任何端点永不回传口令本体**;备份口令(`passphrase`)仅存在于请求体,绝不落盘 / 入库 / 入日志 / 响应。
> **不含**:定时 / 自动备份触发(无调度端点)· 远端备份删除 · 备份内容的分模块读取。

---

## 1. 资源与端点总览

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| GET | `/sync/webdav-config` | 读取 WebDAV 连接配置(A2) |
| PUT | `/sync/webdav-config` | 创建 / 覆盖连接配置(A1;单例 upsert) |
| DELETE | `/sync/webdav-config` | 清除配置连同已存口令密文(A4) |
| POST | `/sync/test` | 测试连接:可达 + 凭据有效 + 远程目录可写(A3) |
| POST | `/sync/backup` | 立即备份:整目录快照 → 口令整包加密 → 上传(B1) |
| GET | `/sync/backups` | 列出远端备份文件(B2) |
| POST | `/sync/restore` | 从远端备份覆盖式恢复(C1–C3) |

> 配置为**单例**(database/sync §2):无 id 路径参数、无列表;PUT 语义为 upsert(首次 = 创建,再次 = 覆盖),故无 POST。

---

## 2. 端点详情

### 2.1 `GET /sync/webdav-config` — 读取配置(A2)

- 200 → `SyncConfigView`;**未配置不报错**,返回 `configured: false` 且其余字段为 `null` / `false`(单例可能不存在,前端据此显隐表单与操作区)。
- 永不回传口令本体;口令状态仅以 `passwordSet: boolean` 表达。

### 2.2 `PUT /sync/webdav-config` — 保存配置(A1)

```ts
interface PutSyncConfigRequest {
  serverUrl: string;        // 服务器地址(http/https,不含备份目录;末尾斜杠归一)
  remoteDir?: string;       // 远程目录;缺省 `autohttps/`;拒绝 `..` 与反斜杠(防路径穿越,DEC6)
  username: string;
  password?: string;        // 口令三态:缺省 = 保留已存;"" = 清除(并删除旧密文);非空 = 重写为新引用
}
```

- **行为**:归一校验 → 口令按三态经 SecretStore 处理(新口令换新 `password_ref` 并删除旧密文)→ upsert 单例 → **200** + `SyncConfigView`。
- **口令永不回显**:保存后读取只见 `passwordSet`。
- **入参校验**:`serverUrl` 非 http(s)、空用户名、`remoteDir` 含 `..` / `\` → `400 validation_failed`。
- 保存**不隐式测试连接** —— 连接有效性由 operator 显式 `POST /sync/test` 确认(A3),与「手动触发」定位一致(DEC1)。

### 2.3 `DELETE /sync/webdav-config` — 清除配置(A4)

- **行为**:删除单例行 + 已存口令密文(`password_ref` 指向的 `.age`)→ **200** `{ "ok": true }`。
- **幂等**:未配置时调用同样成功(无 404)。
- **不动远端**:远端已有备份文件不受影响(远端只增,PRD §2「不含」)。

### 2.4 `POST /sync/test` — 测试连接(A3)

- **行为**:读已存配置(含口令引用解密)→ 对远端目录做可达 + 鉴权 + 可写校验(幂等建目录)→ **200** `{ "ok": true }`。
- 无请求体;测的是**已保存的配置**,不是表单草稿(前端先 PUT 再 test)。
- 失败按远端性质映射:`sync_unreachable` / `sync_auth_failed` / `sync_remote_error`(§4)。

### 2.5 `POST /sync/backup` — 立即备份(B1)

```ts
interface BackupNowRequest {
  passphrase: string;       // 备份加密口令,≥10 位;口令即私钥最后防线(DEC3);不落盘/入库/入日志
}
```

- **行为**:一致性库快照(WAL checkpoint → VACUUM INTO)+ 全部密钥材料(`secrets/*.age` + `master.key`)+ `manifest.json` → zip → **口令整包加密(age passphrase)** → 上传远端目录,文件名 `autohttps-backup-<时间戳>.age` → 回写 `last_backup_*` 留痕(B3)→ **200** + `RemoteBackupItem`(刚上传的文件)。
- `passphrase` 长度 < 10 → `400 validation_failed`(上传前拦截,DEC3)。
- 未配置 WebDAV → `409 sync_not_configured`;远端不可达 / 凭据失效 → §4 对应码;失败同样留痕 `last_backup_result = failed` + `last_backup_error`。

### 2.6 `GET /sync/backups` — 远端备份列表(B2)

- **行为**:PROPFIND 远端目录,按备份文件名前缀(`autohttps-backup-`)过滤 → **200** + `RemoteBackupItem[]`,按修改时间新在前。
- 远端目录尚不存在(从未备份)→ `409 sync_remote_not_found`;未配置 → `409 sync_not_configured`。
- 列表由 WebDAV 服务端实时返回,本端不缓存(DS4)。

### 2.7 `POST /sync/restore` — 覆盖式恢复(C1)

```ts
interface RestoreRequest {
  remoteName: string;       // 远端备份文件名(取自 GET /sync/backups 的 name,防路径穿越)
  passphrase: string;       // 该备份当时的加密口令
}
```

- **行为**(顺序即保障,DEC5):
  1. 下载远端文件并**在内存中解密解析**(manifest 版本校验)→ 口令错 / 包损坏在**写盘前**拦截;
  2. 恢复前把当前库在线导出归档(`pre-restore.db`,可回滚,C2);
  3. 以备份整体覆盖:库逐表替换(按列交集,兼容跨版本备份)+ 密钥材料全量替换(清孤儿、换 master.key、清身份缓存,C3);
  4. 库内悬空口令引用对账置空(提示 operator 重存,C3);
  5. → **200** + `RestoreOutcome`(`requiresRestart: true`)。
- **生效时机**:进程持有库连接与密钥缓存,恢复后**必须重启应用**才生效(DEC5);契约以 `requiresRestart` 显式告知,前端引导重启。
- `remoteName` 不在远端 / 非备份前缀 → `409 sync_remote_not_found`;`passphrase` 解不开 → `422 sync_passphrase_wrong`;未配置 → `409 sync_not_configured`。

---

## 3. 关键 DTO(camelCase)

```ts
interface SyncConfigView {
  configured: boolean;             // 是否已保存配置(单例存在)
  serverUrl: string | null;        // 服务器地址(展示/回填;不含远程目录)
  remoteDir: string | null;        // 远程目录(展示/回填)
  baseUrl: string | null;          // 拼好的完整远端目录 URL(serverUrl + remoteDir;实际请求目标)
  username: string | null;
  passwordSet: boolean;            // 口令是否已保存;本体永不回传
  lastBackupAt: string | null;     // 上次备份时间(RFC3339;B3 留痕)
  lastBackupResult: string | null; // "success" | "failed"
  lastBackupError: string | null;  // 上次失败原因(成功时为 null)
}

interface RemoteBackupItem {
  name: string;                    // 远端文件名(autohttps-backup-<时间戳>.age)
  size: number | null;             // 字节数
  modified: string | null;         // 远端修改时间(RFC3339)
}

interface RestoreOutcome {
  restoredFrom: string;            // 恢复来源(远端文件名)
  backupCreatedAt: string;         // 备份生成时间(manifest 记录,RFC3339)
  secretsRestored: number;         // 替换的密钥材料件数
  requiresRestart: boolean;        // 恒 true:恢复后须重启应用生效(DEC5)
}
```

> 三个写入口令字段(`PutSyncConfigRequest.password` / `BackupNowRequest.passphrase` / `RestoreRequest.passphrase`)**只进不出**:任何响应 DTO 均不含口令 / 口令明文派生字段;日志同样脱敏(project §7)。`SyncConfigView` 不含 `password_ref` 本体(引用是内部实现细节,database 层)。

---

## 4. 错误码清单(snake_case 领域码)

| code | HTTP | 触发 |
| --- | --- | --- |
| `sync_not_configured` | 409 | test / backup / backups / restore 时无已存配置(前置缺失) |
| `sync_remote_not_found` | 409 | 远端目录不存在(backups)/ 指定 `remoteName` 不存在或非备份前缀(restore) |
| `sync_passphrase_wrong` | 422 | 备份口令错误或包损坏(内存解析阶段拦截,写盘前) |
| `sync_unreachable` | 502 | 连接超时 / 网络不可达(下游错误) |
| `sync_auth_failed` | 502 | WebDAV 凭据无效或无权限(上游 401/403 语义 → 下游 502) |
| `sync_remote_error` | 502 | 远端其他错误(WebDAV 服务端 5xx / 协议异常) |

> 结构 / 类型非法(`serverUrl` 非 http(s)、`remoteDir` 含 `..` / `\`、`passphrase` < 10 位、缺必填字段)→ 全局 `validation_failed`(400)。远端类三码(`sync_unreachable` / `sync_auth_failed` / `sync_remote_error`)为 **502 下游错误**语义:本端正常,问题在 WebDAV 侧或链路。

---

## 5. 状态机与 SSE

- **无独立状态机**:配置为长存单例(无生命周期);备份 / 恢复为 operator 同步动作(请求-响应内完成),不建任务行、不进任务中心(DEC1 手动定位)。
- **本模块不发 SSE 事件**:备份 / 恢复结果随响应返回,前端 react-query 于 mutation 成功即失效 `/sync/webdav-config` 与 `/sync/backups`;`last_backup_*` 留痕经 GET 读取。
- 恢复完成后的**生效**依赖应用重启(DEC5),非 SSE 驱动;前端据 `RestoreOutcome.requiresRestart` 提示。

---

## 6. PRD/DB 缺口(architect 停止条件核查)

- 无阻塞缺口。功能 A1–A4 / B1–B3 / C1–C3 均有端点承载;口令三态(保留 / 清除 / 重写)与「读取永不回传」在 PRD §2-A1 明确;`last_backup_*` 留痕字段由 database/sync §2 承载;远端过滤前缀、文件名时间戳格式、`passphrase` 最小长度(10)均有实现 / PRD 依据(DEC3 / DEC6)。
- **提示**:`remoteDir` 默认值在 DTO 注释与 PRD 写作 `autohttps/`(含尾斜杠,展示口径),存储层归一为无尾斜杠(database/sync §2);两端口径差异仅展示层,不影响拼接(`baseUrl` 由服务端拼好返回)。
