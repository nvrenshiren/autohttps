# 技术选型与编码协议 · autohttps

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 技术基线 · 端点: app · 撰写: architect
> 信任基础: 全部 17 份 PRD 已 approved(2026-07-16);配套 [ARCHITECTURE.md](./ARCHITECTURE.md)。
> 定位: 本文件定"用什么技术 + 编码协议"。§1 给技术栈、§2 给逐项决策清单(推荐 + 备选 + 理由);**§2 决策 1–10 已于 2026-07-16 由 orchestrator 全部确认(采纳全部推荐),下文措辞已相应落定**;§3 编码协议**一旦批准即定死、后续不得漂移**;§4 定枚举/字典单一真相机制(architect 唯一变更入口)。
> 版本核实: 本文所有版本号为 **2026-07-16 经 crates.io / npm registry 实测的当前稳定版**(本会话未接入 context7 MCP,改用官方 registry 核实,详见 §6)。基线只锚定**大版本 + 选型**;精确锁版在实现期定。

---

## 1. 技术选型总表(推荐栈)

> 下表是**最终栈**;凡带"✅ 决策"标记的项曾有实质权衡,**已于 2026-07-16 由 orchestrator 确认(采纳推荐)**,详见 §2。未标记项为低风险常规选择。

### 1.1 语言与运行时

| 维度 | 推荐 | 版本(2026-07-16) | 说明 |
| --- | --- | --- | --- |
| 后端语言 | Rust | 1.95(工具链实测) | project §4 已定"同一 Rust 核心";跨平台、内存安全、生态齐备 |
| 异步运行时 | tokio | 1.52 | ACME 在线交互 / 任务执行 / axum 均异步;事实标准 |
| 前端语言 | TypeScript | **5.8(已确认稳妥线,不采 7.0)** | React 生态标准;已确认(orchestrator, 2026-07-16)采用 5.8 稳妥线——7.0 原生编译器太新、ts-rs 产物 / Vite 8 / shadcn 工具链跟进不确定;`tsc --noEmit` 作为 build 前置检查(§2-决策9) |
| 前端框架 | React | 19.2 | project §4/§9-D1 已定"同一 React + Vite 前端" |
| 前端构建 | Vite | 8.1 | project §4/§9-D1 已定;当前主版 8.x |

### 1.2 后端关键库

| 用途 | 推荐 | 版本 | 备选 | 标记 |
| --- | --- | --- | --- | --- |
| Web / HTTP 服务 | axum | 0.8 | actix-web / warp | 承 ARCHITECTURE §3 统一 HTTP 传输 |
| HTTP 中间件 / 静态资源 | tower-http | 0.7(tower 0.5) | — | ServeDir / CORS / 压缩 |
| SPA 内嵌打包 | rust-embed | 8.12 | tauri resource | 前端产物内嵌进可执行文件、离线自包含 |
| SQLite 访问层 | **SeaORM** | 1.1(sea-orm-migration 1.1) | sqlx 0.9 / rusqlite 0.40 / diesel 2.3 | ✅ 决策2(已确认) |
| ACME 客户端 | **instant-acme** | 0.8 | acme2 / rustls-acme | ✅ 决策4(已确认) |
| X.509 / 自签 CA | **rcgen** | 0.14 | openssl / rustls 原语 | ✅ 决策5(已确认) |
| 证书/密钥解析(导入根 CA) | x509-parser + rustls-pemfile | 0.18 / 2.2 | — | 校验导入的根 CA 证书 + 私钥配对 |
| 敏感数据静态加密 | **age** | 0.12 | aes-gcm 0.11 + argon2 0.5 | ✅ 决策3(已确认·加密基线) |
| OS keychain | keyring | 4.1 | — | ✅ 决策3(已确认·桌面加固) |
| DNS-01 本地预检 | hickory-resolver | 0.26 | — | acme flow §4.3 可选提交前预检 TXT 是否生效 |
| Rust→TS 类型/枚举生成 | **ts-rs** | 12.0 | specta 1.0 + specta-typescript 0.0.12 (+ tauri-specta 1.0) | ✅ 决策6(已确认,随决策1-A) |
| 时间 | time | 0.3 | chrono 0.4 | RFC3339 UTC;与 rustls/x509 生态对齐 |
| 序列化 | serde | 1.0 | — | — |
| 错误处理 | thiserror + anyhow | 2.0 / 1.0 | — | 库用 thiserror 定类型化错误、bin 用 anyhow |
| 日志 / 追踪 | tracing | 0.1 | — | 结构化日志;敏感数据脱敏(tasks DS2) |
| 实体 ID | uuid | 1.24 | 自增整型 | ✅ 决策10(已确认·UUIDv7) |

### 1.3 桌面形态(Tauri v2)

| 用途 | 推荐 | 版本 | 说明 |
| --- | --- | --- | --- |
| 桌面框架 | Tauri | 2.11(@tauri-apps/api 2.11 / cli 2.11) | project §4 已定 Tauri;v2 稳定、跨三平台、托盘/自启/单实例插件齐备 |
| 单实例 | tauri-plugin-single-instance | 2.4 | 防同一形态跑两份争同库 |
| 开机自启 | tauri-plugin-autostart | 2.5 | settings 开机自启开关(仅桌面) |
| 原生对话框 / 文件 | tauri-plugin-dialog / -fs | 2.7 / 2.5 | 导出私钥/证书/根 CA 走原生保存 |
| 托盘角标 / 通知 | Tauri 内建 tray + tauri-plugin-notification | 2.3 | dashboard 红点托盘角标(仅桌面) |

### 1.4 前端关键库(完整前端栈)

> 前端栈由用户敲定(2026-07-16);shadcn 现行约定经 Context7 核实(shadcn 自 2025-02 全面支持 Tailwind v4 + React 19;lucide 为默认图标;自带 toast 已废弃改 sonner)。版本为 2026-07-16 npm registry 实测当前版。

| 用途 | 推荐 | 版本 | 说明 |
| --- | --- | --- | --- |
| UI 组件 | shadcn/ui + Radix UI primitives | 源码(radix-ui 1.6) | **手写源码于 `frontend/src/components/ui/`,非 CLI 生成**;须遵 shadcn v4 / React 19 现行约定:`data-slot` 属性、无 `forwardRef`、`@theme inline` 的 OKLCH CSS 变量 |
| 样式 | Tailwind CSS v4 + @tailwindcss/vite | 4.3 | `@theme` token、CSS-first、OKLCH;`@theme` 的 `--color-*` 即设计系统 token 落点 |
| 图标 | lucide-react | 1.24 | shadcn 默认图标 |
| 服务端状态 / 数据获取 | @tanstack/react-query | 5.101 | 列表/详情缓存、后台重取、失效重取;配 SSE `invalidate` 刷新红点 / 列表 |
| 客户端状态 | zustand | 5.0 | **仅管客户端 / UI 态**(向导步骤、非 URL 临时筛选、SSE 连接状态);**不承载服务端状态**(归 react-query),禁止复制服务端数据 |
| 路由 | react-router | 7+(当前 8.x) | 或 TanStack Router;低风险,designer/dev 可定 |
| 表单 | react-hook-form + @hookform/resolvers(zod) | 7.81 / 5.4 | 签发向导 / ACME 账户 / 设置 / 新增根 CA |
| 数据表格 | @tanstack/react-table | 8.21 | 证书 / 域名 / 任务列表的排序 / 筛选 / 分页;shadcn data-table 基于它 |
| toast | sonner | 2.0 | 替代 shadcn 已废弃自带 toast;签发 / 续签 / 吊销成功失败反馈 |
| 运行时校验 | zod | 4.4 | wire 边界运行时校验;类型来自 core 的 ts-rs 绑定,校验在边界 |
| 类型/枚举来源 | (由 ts-rs 从 core 生成) | — | `frontend/src/bindings/`,勿手写枚举字面量(§4) |

> **归属边界(清晰划分)**:**UI 组件库 / 样式 / 设计 token 的具体设计归 designer 设计系统**——designer 消费 shadcn + Tailwind v4,定 `@theme` 的 token 值(颜色 / 间距 / 圆角等 OKLCH 变量);**前端架构层(react-query ↔ zustand 分工、路由、表单、@tanstack/react-table、ts-rs 绑定消费、sonner 反馈)归本基线**。即:基线定"用哪些库、各司何职",designer 定"长什么样"。

---

## 2. 关键选型决策清单(逐项:推荐 · 备选 · 理由 · 已确认)

> 每项均**已确认(orchestrator, 2026-07-16),全部采纳 architect 推荐**。以下保留推荐理由与备选(备选留作记录,不删);每项末行 **"已确认"** 为最终取定。

### ✅ 决策1 · 两形态前后端传输统一方式(最关键,决定 API 契约形态)

- **推荐**:方案 A · **统一 HTTP/WS**——两形态跑同一 axum 服务,桌面内嵌回环服务,前端始终走 HTTP(REST)+ SSE/WS。
- **备选**:方案 B · **双传输**——桌面 Tauri IPC(`invoke`)+ 服务器 HTTP,客户端抽象层屏蔽。
- **理由**:A 让前端只有一层网络代码、API 契约只定义一次、实时推送统一,直接落实 D1、最小化契约漂移;B 需两套传输 + 两套绑定、契约实质重复(D1 所警惕)。详见 [ARCHITECTURE §3]。
- **已确认(orchestrator, 2026-07-16):方案 A · 统一 HTTP/WS**(桌面内嵌回环)。API 契约据此按 REST + SSE/WS 形态展开,不再切换;方案 B 留作记录。

### ✅ 决策2 · 本地持久化访问层

- **推荐**:**SQLite + SeaORM 1.x**(sea-orm + sea-orm-migration,均 1.1,2026-07 仍在活跃发版)。
- **备选**:① sqlx 0.9(SQL 优先、编译期校验、更轻);② rusqlite 0.40(同步、最轻、需 spawn_blocking);③ diesel 2.3(同步、编译期安全,与 tokio 异步不贴合)。
- **理由**:实体含证书/域名/账户/挑战/根 CA/任务 + 域名↔证书多对多(SAN)+ 证书↔任务一对多等中等关系;SeaORM 提供实体建模 + 关系 + 内建迁移框架 + 异步(贴合 tokio),作为"schema 单一真相"表达力强、已达 1.x 稳定。sqlx 更轻且编译期校验 SQL,但关系需手写、编译期校验需离线缓存或活库——若团队偏好 SQL 优先则选它。
- 权衡:SeaORM(建模省心、层更厚)vs sqlx(更贴 SQL、更轻、编译期校验)。**已确认(orchestrator, 2026-07-16):SeaORM 1.x**;sqlx / rusqlite / diesel 留作备选记录。
- 注:两形态各自独立库(project §4),即每实例一个 SQLite 文件,落 settings 数据存储路径下,启用 WAL。

### ✅ 决策3 · 敏感数据静态存储(私钥 / ACME 账户密钥 / 根 CA 私钥)

- **推荐**:**数据目录内加密静态存储(可移植基线)** + 桌面可选叠加 OS keychain 加固。
  - 密钥材料以密文落 settings 数据存储路径下;SQLite 仅存"存储位置引用",**绝不明文入库/入日志**(certificates DS6 / acme DS1 / local-ca DS2 / tasks DS2 脱敏)。
  - 加密:推荐 **age**(X25519 + ChaCha20-Poly1305,API 简洁);或 AES-256-GCM(aes-gcm)+ 随机数据密钥。主密钥以严格文件权限(Unix 0600 / Windows ACL)保护;有 keychain 时(桌面)可把主密钥封存进 keychain 作加固层。
- **备选**:**纯 OS keychain(keyring 4.x)**——Windows Credential Manager / macOS Keychain / Linux Secret Service。
- **理由 / 关键权衡**:
  - keychain 在**桌面**体验最佳(OS 级保护、无需自管密钥),但 **Linux Secret Service 依赖 D-Bus/gnome-keyring 等常驻服务,无头服务器常不具备**——这与**服务器形态 + 7×24 无人值守自恢复重启 + 离线**直接冲突(重启时无法弹口令、无 keyring 守护)。
  - 加密静态存储在**全平台(含无头服务器)行为一致、离线、可无人值守重启**,是跨形态的可移植基线;诚实代价:无用户口令/硬件密钥时,磁盘上的主密钥对足够高权限的本地主体可读——这与信任模型一致(D4:信任=本机/网络边界;桌面=单本机用户,服务器=可信主机)。
- 硬约束:满足 project §7(敏感数据安全存储、不明文外泄)+ 跨平台对等 + 离线可用。**已确认(orchestrator, 2026-07-16):加密静态存储(age)为跨形态基线 + 桌面可选叠加 OS keychain 加固**(即本推荐 option 1);纯 keychain 留作备选记录。

### ✅ 决策4 · ACME 客户端库

- **推荐**:**instant-acme 0.8**。
- **备选**:acme2 / acme-lib(较老、维护弱)/ rustls-acme(偏服务器 TLS-ALPN 自动签,不贴合手动 HTTP-01/DNS-01)。
- **理由**:instant-acme 是 Rust 事实标准的**异步 ACME v2 客户端**(rustls 团队系),覆盖 order→challenge→finalize,支持 HTTP-01 + DNS-01,挑战就绪时机由调用方控制——正好承接 acme flow 的**手动 DNS-01**(展示 TXT→等待→用户确认→再请求校验,挑战停在 `awaiting_manual`)。活跃维护、采用度高。
- **已确认(orchestrator, 2026-07-16):instant-acme 0.8**。风险低;备选留作记录。

### ✅ 决策5 · X.509 / 自签根 CA 库

- **推荐**:**rcgen 0.14**(生成自签根 CA + 用根 CA 签发内网叶子证书 + SAN + 自定义有效期)+ x509-parser/rustls-pemfile(导入根 CA 时解析校验)。
- **备选**:openssl crate(功能全,但引入原生 OpenSSL 构建依赖,跨平台尤其 Windows 构建痛,损"三平台对等")。
- **理由**:rcgen 纯 Rust(底层 aws-lc-rs/ring)、跨平台构建无痛、直接支持 CA 签名与 SAN,契合 local-ca 本地离线签发(project §7)。**避免 openssl 原生依赖**以保跨平台对等。
- **已确认(orchestrator, 2026-07-16):rcgen 0.14 + x509-parser,避免 openssl 原生依赖**;openssl 备选留作记录。
- 注(作废机制):rcgen 不产出 CRL/OCSP;local-ca 的"内网证书作废标记"(§3.5 / DS3)在 MVP 落为**本地作废记录**(根 CA 名下的已作废序列号清单,属数据),非签发标准 CRL——与 PRD"作废记录 / 作废清单"口径一致。CRL/OCSP 生成后置。

### ✅ 决策6 · Rust↔TS 类型/枚举生成(与决策1耦合)

- **推荐**(配决策1-A):**ts-rs 12**——`#[derive(TS)]` 把 core 的结构体/枚举(含 5 台状态机枚举)导出为 `.ts`,传输无关,覆盖 REST/WS 的 DTO + 共享枚举。
- **备选**(配决策1-B):**specta 1.0 + specta-typescript 0.0.12 + tauri-specta 1.0**——与 Tauri IPC 配套生成强类型命令绑定;若走双传输更划算。
- **理由**:统一 HTTP 传输下类型需覆盖 REST/WS DTO 且传输无关,ts-rs 更贴切且版本线干净(稳定 v12);specta 在 Tauri IPC 场景最亮眼但 v2 生态版本较碎(specta/specta-typescript 处 0.0.x)。**决策6 跟随决策1**:选 A 用 ts-rs,选 B 用 specta 系。
- **已确认(orchestrator, 2026-07-16):ts-rs 12**(随决策1-A);specta + tauri-specta 留作备选记录。

### ✅ 决策7 · 任务队列 + 崩溃恢复策略

- **推荐**:**自建轻量队列**——以任务表(SQLite)为持久队列,tokio 有界并发 worker 执行;崩溃恢复取"启动时 `running`→`failed`(可重试)+ `queued` 重排 + 证书扫描据实校正"。
- **备选**:引入 apalis 0.7 等任务框架(内建重试/调度/多后端)。
- **理由**:PRD 明确"任务不持重试次数/间隔、自动再尝试依附扫描 + settings 自动续签开关"(SF2 / DT5),apalis 的内建重试/调度模型与此相抵,反增适配与双策略源风险;自建队列天然满足"队列即历史表、崩溃可恢复、不引外部中间件",契合 tasks §3.3 底线("不卡死、可被 operator 看到并处置")。
- **已确认(orchestrator, 2026-07-16):自建轻量队列(SQLite 表即队列)**,恢复策略取"running→failed 可重试 + queued 重排 + 证书扫描据实校正";细节(能否续跑在途)实现期可再定,但**不得违背"不卡死"底线**。apalis 留作备选记录。

### ✅ 决策8 · dashboard 红点实时刷新

- **推荐**:**SSE 服务端推送 + 轮询兜底**。
- **备选**:WebSocket(双向,后续需前端→服务端流式再上)/ 纯轮询(最简、实时差、空转多)。
- **理由**:红点/待处理/DNS-01 等待/任务进度都是**单向**(服务端→前端)推送,SSE 原生 EventSource + 自动重连 + axum 实现简单即足;WS 的双向能力 MVP 用不上。两形态统一走同一 axum 服务,行为一致。详见 [ARCHITECTURE §9.2]。
- **已确认(orchestrator, 2026-07-16):SSE 推送 + 轮询兜底**;WebSocket / 纯轮询留作备选记录。

### ✅ 决策9 · TypeScript 版本线(低风险)

- **原推荐**:当前稳定 TypeScript(2026-07 主版 **7.0**,原生编译器、极快),存疑回退 5.8。
- **备选(即最终所选)**:**5.8 稳妥线**——成熟、工具链全面兼容。
- 理由:7.0 是原生移植编译器、构建/检查显著更快,但属很新的大版本;ts-rs 产物 / Vite 8 / shadcn 工具链对 7.0 的跟进尚不确定。
- **已确认(orchestrator, 2026-07-16):TypeScript 5.8 稳妥线(不采 7.0)**——用户就此项改选稳妥线;`tsc --noEmit` 作为 build 前置检查(typecheck 步骤)。低风险。

### ✅ 决策10 · 实体主键 ID(低风险)

- **推荐**:对外暴露的领域实体主键用 **UUIDv7 文本 ID**(时间可排序、不透明、不可枚举、利于未来导出)。
- **备选**:自增整型(单实例本地库足够,但可枚举、对外略透明)。
- 理由:两形态各自独立库不互相同步,自增整型技术上够用;UUIDv7 更适合 API 对外(opaque)且导出友好。**已确认(orchestrator, 2026-07-16):UUIDv7 文本 ID**;自增整型留作备选记录。低风险。

---

## 3. 编码协议(批准即定死,后续不得漂移)

> 承 architect 红线:API 风格、分页参数、错误码规范等一旦定死不得漂移;能机器查的沉淀为 §5 protocolLints。以下协议**以决策1-A(统一 HTTP)为前提**;若决策1 改选 B,IPC 侧命令契约另议,但命名/枚举/错误码/时间戳/ID 协议仍适用。

### 3.1 API 风格

- **REST over HTTP + JSON**,资源导向;实时经 **SSE**(备选 WS)。端点只有一个 `app`,故 API 契约文档落 `docs/architecture/api/app/{模块}.md`(跨模块共用放 `docs/architecture/api/app/common/` 或 `common/`)。
- HTTP 方法语义常规:GET 查、POST 建/触发动作、PATCH 改、DELETE 删。证书的签发/续签/吊销/重试/取消等**动作**用 POST 子资源(如 `POST /certificates/{id}/renew`),不硬套 CRUD。
- 具体路由/资源/请求响应体在各模块 API 文档定,本基线只锚定风格;**不在此设计各模块 API**(不越界)。

### 3.2 命名与 JSON 序列化约定

| 面 | 约定 |
| --- | --- |
| Rust 类型 | 类型 `PascalCase`、字段 `snake_case`(Rust 惯例) |
| JSON 字段名 | **`camelCase`**(serde `rename_all = "camelCase"`,前端 TS 友好) |
| **枚举 wire 值** | **严格等于 PRD 指定的 `snake_case` 标识**(如 `pending_issue` / `awaiting_manual` / `renewal_failed` / `active`);serde `rename_all = "snake_case"` 于枚举。枚举值是领域常量,**不 camelCase 化、不翻译、不缩写**(见 §4) |
| 路由路径 | 资源用复数小写(`/certificates` `/domains` `/acme/accounts` `/root-cas` `/tasks`),多词用 `kebab-case` |
| 查询参数 | `camelCase`(与 JSON 字段一致) |

### 3.3 分页协议(统一)

列表接口(tasks/certificates/domains 等,tasks 历史只增需分页)统一:

- 请求:`?page=<从1起>&pageSize=<默认20,上限100>`,可选 `sort=<字段>&order=<asc|desc>` 及模块级过滤参数。
- 响应包络:`{ "items": [...], "page": 1, "pageSize": 20, "total": <总数> }`。
- 风格取 **limit/offset(page/pageSize)**——本地 SQLite 中等数据量足够、实现最简。(备选游标分页用于超大只增列表,若 tasks 历史体量成问题再增量;属可后续调整项。)

### 3.4 错误码规范(统一)

- 错误响应包络:`{ "error": { "code": "<稳定机器码>", "message": "<人读信息>", "details": <可选对象> } }`。
- `code`:**稳定的 `snake_case` 领域错误码**(如 `cert_not_found` / `domain_in_use` / `acme_account_not_registered` / `root_ca_expired` / `challenge_timeout` / `validation_failed`),在 core 单一定义(枚举)并导出;前端按 `code` 分支处置,`message` 仅供展示、可变。
- HTTP 状态映射常规:400 入参非法、404 不存在、409 状态冲突(如对进行中态证书直接删除)、422 业务规则拒绝、500 内部错误。具体 `code` 清单在各模块 API 文档逐一登记(本基线只定包络与命名规则)。

### 3.5 时间与 ID

- **时间**:wire 一律 **RFC3339 / ISO-8601 UTC 字符串**(如 `2026-07-16T08:00:00Z`);Rust 侧用 `time`(或 chrono)。有效期、入队/开始/结束时间等均遵此。
- **ID**:见决策10;对外 ID 不透明、稳定,证书删除后 tasks 仍保留对其只读引用(tasks DEC3),故实体 ID 不复用。

### 3.6 运行形态标志

- 前端经 API 取"当前运行形态"(`desktop` / `server`,settings DS5)以做仅桌面/仅服务器显隐;该值由运行载体探测、非可切换配置(settings SF4)。

---

## 4. 共享枚举与字典:单一定义位置 + 跨端同步机制(architect 唯一变更入口)

> architect 红线:枚举禁止硬编码字符串字面量散落各端;**你是唯一变更入口**。developer 缺枚举须停下等 architect,不得自行加。

### 4.1 单一定义位置

- 全部共享枚举/字典在 **`crates/core/src/domain/enums.rs`** 单一定义(Rust `enum`),派生 `#[derive(Serialize, Deserialize, TS)]`,serde `rename_all = "snake_case"` 使 wire 值等于 PRD 标识。

### 4.2 跨端同步机制(Rust 定义 → 导出 TS)

- 经 **ts-rs**(决策6-A)把 core 枚举/DTO 导出到 **`frontend/src/bindings/`**;前端只从该目录 import 枚举与类型,**严禁手写状态字符串字面量**。
- 生成产物纳入版本库或构建期生成(实现期定);任一枚举变更 = 改 core `enums.rs` → 重新导出 → 前端类型自动更新。**Rust 是唯一真相,TS 是投影。**
- 若决策1 改选 B,则改用 specta + tauri-specta 生成绑定(机制同:core 单一定义 → 生成 TS),枚举真相仍在 core。

### 4.3 必须单一定义的枚举清单(标识严格照 PRD,不得漂移)

| 枚举 | 归属 | wire 值(snake_case,照 PRD) |
| --- | --- | --- |
| 证书状态 | certificates(flows/certificates §2.1) | `pending_issue` `issuing` `issue_failed` `valid` `expiring_soon` `renewing` `renewal_failed` `expired` `revoking` `revoked` |
| 任务状态 | tasks(flows/tasks §3.1) | `queued` `running` `succeeded` `failed` `cancelled` |
| 任务类型 | tasks(§2.1) | `issue` `renew` `revoke` |
| 任务触发方式 | tasks(§2.2) | `manual` `auto` `cleanup` |
| ACME 账户状态 | acme(flows/acme §2.1) | `unconfigured` `registering` `registered` `registration_failed` |
| 验证挑战状态 | acme(flows/acme §3.1) | `pending` `awaiting_manual` `validating` `passed` `failed` `cancelled` |
| 验证方式类别 | acme/domains | `http_01` `dns_01`(对应 glossary HTTP-01(webroot) / DNS-01(手动);wire 标识用 snake_case,展示名归前端 i18n) |
| 根 CA 状态 | local-ca(flows/local-ca §2.1) | `active` `expired` |
| 签发方式 | certificates(DS3) | `acme` `self_signed`(公共 ACME / 自签根 CA) |
| 运行形态 | settings(DS5) | `desktop` `server` |
| 错误码 | 全局(§3.4) | 领域 `snake_case` 码,逐模块登记 |

> 上表是**基线锚定的枚举真相入口**;后续任一模块新增/变更枚举值,均须经 architect 改 `enums.rs` 并同步本表 + 重新导出 TS。**多端漂移的唯一防线是这个单一入口。**

---

## 5. protocolLints 提议(本次只提议,不强制启用)

> 能机器查的约定沉淀为 `workbench.config.json` 的 `protocolLints`,违例在 complete 时被机器拦截。以下为**候选**,基线批准后与 orchestrator 商定是否启用(当前 `machineChecks.enabled=false`)。

| # | 约定 | 机器检查思路 |
| --- | --- | --- |
| L1 | 枚举字面量不散落 | 前端源码(`frontend/src` 除 `bindings/`)不得出现状态枚举的裸字符串(如 `"pending_issue"`);须 import 自 `bindings/` |
| L2 | JSON 字段 camelCase / 枚举值 snake_case | 校验 DTO serde 属性:结构体 `rename_all="camelCase"`、枚举 `rename_all="snake_case"` |
| L3 | 时间为 RFC3339 UTC | API 契约文档/DTO 中时间字段类型统一;禁裸时间戳整数漂移 |
| L4 | 分页参数统一 | 列表接口须含 `page`/`pageSize` 且响应含 `{items,page,pageSize,total}` |
| L5 | 错误包络统一 | 错误响应须为 `{error:{code,message}}`,`code` 为 snake_case |
| L6 | 敏感数据不入日志 | 私钥/账户密钥/根 CA 私钥相关字段不得进 tracing 输出(tasks DS2 脱敏) |
| L7 | 无 openssl 原生依赖 | Cargo 依赖树不含 `openssl`/`openssl-sys`(保跨平台构建,决策5) |

---

## 6. 版本核实说明

- 本会话 **未接入 context7 MCP**(项目 `.mcp.json` 仅配置 opcflow;architect 可调用工具集不含 context7)。为遵守"不凭记忆下版本结论",改用**本机 cargo 1.95 + npm**直接查询 **crates.io / npm registry** 核实当前版本(结果比知识截止 2026-01 更新)。
- **已核实且注意到显著新于旧知识的事实**:Tauri **2.11**、SeaORM 已达 **1.x 稳定**(sea-orm 1.1 / sea-orm-migration 1.1)、sqlx **0.9**、keyring **4.x**、rcgen **0.14**、instant-acme **0.8**、ts-rs **12**、specta **1.0** + specta-typescript **0.0.12**、Vite **8**、React **19.2**、TypeScript **7.0**、Zod **4**、axum **0.8**、rustls **0.23**。
- **与 PRD 约束的冲突核查**:未发现选型与 PRD 冲突。关键印证——instant-acme 支持"调用方控制挑战就绪时机" ⇒ 可承接手动 DNS-01 的 `awaiting_manual` 长停留(acme DA2);rcgen 纯 Rust ⇒ 满足跨平台对等 + 本地离线签发(project §7);加密静态存储路线 ⇒ 满足无头服务器 + 离线 + 无人值守重启(否则 Linux Secret Service 依赖 D-Bus 会破服务器形态)。
- 精确锁版(含 patch)在实现期由 developer 于 Cargo.lock / package-lock 固定;基线只锚定大版本 + 选型。

---

## 7. 决策记录(append-only)

> 只增不改;记"定了什么 / 为什么不做另一选项"。本文 §2 决策清单已由 orchestrator 于 2026-07-16 全部确认(见 AR7);AR1–AR6 为 architect 在基线中已锚定、低争议的技术决定。

- **AR1(2026-07-16)· 三层 crate 分离**:`crates/core`(业务真相)/ `crates/api`(传输契约)/ `crates/{server,desktop}`(形态宿主)。为什么:让"业务只写一遍、传输定义一遍、形态各自装配",两形态共享最大化、漂移最小化(承 D1)。
- **AR2(2026-07-16)· 枚举单一真相在 core、经生成投影到 TS**:全部共享枚举定义于 `core/domain/enums.rs`,wire 值严格照 PRD snake_case 标识,导出 TS 供前端消费,禁手写字面量。为什么:枚举散落各端是多端漂移主因;单一入口 + 生成投影是唯一可机器约束的防线(§5-L1)。
- **AR3(2026-07-16)· 避免 openssl 原生依赖**:X.509/CA 用纯 Rust rcgen 系,不引 openssl crate。为什么:保 Windows/Linux/macOS 三平台构建对等(project §7),openssl 原生构建在 Windows 尤其痛。
- **AR4(2026-07-16)· 敏感数据绝不明文入库/入日志**:仅存"存储位置引用",密钥材料密文落数据目录(或 keychain);tracing 脱敏。为什么:project §7 数据安全硬约束 + tasks DS2 明示日志脱敏。
- **AR5(2026-07-16)· 任务表即持久队列、无外部队列中间件**:任务状态机表承担队列 + 历史双职责。为什么:天然持久化 + 崩溃可恢复 + 契合"任务不持独立重试参数、自动再尝试依附扫描"(SF2/DT5),避免引入与 PRD 相抵的框架内建重试。
- **AR6(2026-07-16)· 编码协议定死项**:JSON camelCase / 枚举 snake_case / limit-offset 分页 / `{error:{code,message}}` 错误包络 / RFC3339 UTC 时间。为什么:承 architect 红线"协议定死不得漂移",并可沉淀为 protocolLints 机器拦截(§5)。
- **AR7(2026-07-16)· §2 决策清单最终取定(orchestrator 拍板,全部采纳 architect 推荐)**:
  - 决策1 传输 = **方案 A 统一 HTTP/WS**(桌面内嵌回环)⇒ API 契约为 REST + SSE/WS。
  - 决策2 持久化 = **SQLite + SeaORM 1.x**。
  - 决策3 敏感存储 = **加密静态存储(age)为跨形态基线 + 桌面可选叠加 OS keychain 加固**。
  - 决策4 ACME = **instant-acme 0.8**。
  - 决策5 X.509/CA = **rcgen 0.14 + x509-parser,避 openssl 原生依赖**。
  - 决策6 类型生成 = **ts-rs 12**(随决策1-A)。
  - 决策7 任务队列 = **自建轻量队列(SQLite 表即队列)+ 崩溃恢复"running→failed 可重试 + queued 重排 + 扫描校正"**。
  - 决策8 实时刷新 = **SSE 推送 + 轮询兜底**。
  - 决策9 TS 版本 = **TypeScript 5.8 稳妥线(不采 7.0;`tsc --noEmit` 作 build 前置检查)**(用户就此项改选稳妥线,见 AR8)。
  - 决策10 实体 ID = **UUIDv7 文本 ID**。
  - 为什么记此一条:让批准后的基线不含悬空待决标记;各决策的备选与理由仍保留在 §2 供追溯,备选不删、仅落定取向。
- **AR8(2026-07-16)· 前端 UI 栈敲定(用户拍板)**:并入 §1.4——UI 组件 **shadcn/ui**(手写源码于 `frontend/src/components/ui/`,遵 v4/React19 约定:`data-slot`、无 forwardRef、`@theme inline` OKLCH)+ **Radix** primitives;样式 **Tailwind CSS v4**(`@theme` token、CSS-first、OKLCH);图标 **lucide-react**;服务端状态 **@tanstack/react-query**;客户端状态 **zustand**;路由 **react-router**;表单 **react-hook-form + zod resolver**;数据表格 **@tanstack/react-table**;toast **sonner**;校验 **zod 4**。
  - **react-query ↔ zustand 分工(定死)**:服务端数据(证书 / 域名 / 任务 / 账户等)一律走 react-query(缓存 + SSE invalidate 刷新);zustand **仅**管客户端 / UI 态(向导步骤、临时筛选、SSE 连接状态),**禁止复制服务端数据**。为什么:混用会制造"服务端数据两处副本"的漂移,与 §4 单一真相精神一致。
  - **决策9 改选 5.8**:7.0 原生编译器太新、ts-rs 产物 / Vite 8 / shadcn 工具链跟进不确定,用户就此改选 TypeScript 5.8 稳妥线,`tsc --noEmit` 作 build 前置检查(AR7 决策9 已同步)。
  - **归属边界**:UI 组件 / 样式 / 设计 token 的具体设计归 designer 设计系统(消费 shadcn + Tailwind v4、定 `@theme` token 值);前端架构层(库选型与各司其职、react-query/zustand 分工、ts-rs 绑定消费)归本基线(§1.4)。
