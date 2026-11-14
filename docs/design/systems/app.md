# App UI 补充规范

> 文档状态: draft · 端点: app
> 当前品牌、Token、主题和组件表达以 [COHESION 接入说明](../cohesion-adoption.md) 与逐科正式设计系统为准。
> 本文只保留 autohttps 的业务状态语义与交互约束，不重复维护一套视觉主题。

## 0. 设计边界

- 页面内容、字段、状态转移和操作范围遵循对应 PRD 与 API 契约。
- 品牌、颜色、字体、圆角、阴影、浮层和动效遵循 COHESION 与已接入组件源码。
- 状态是本产品的重要信息；必须同时提供状态文字，不能只靠颜色表达。
- 等待、失败、加载和禁用状态应如实呈现，不模拟没有业务依据的进度。
- 不添加 PRD 未定义的指标、字段、批量操作和状态。

## 1. 色彩来源

状态与分类样式只使用 COHESION 派生的语义 Token。页面和业务组件不得定义第二套品牌色或复制色值表。状态级别映射见 §3，运行时 Badge 变体由 StatusBadge 统一承接。

## 2. 主题与组件 Token

主题文件、浅深主题入口和基础控件样式以 docs/design/cohesion-adoption.md 及 frontend/src/theme、frontend/src/components/ui 为准。新增组件先复用现有语义 Token 与 shadcn/Radix 组件；不要从旧的 Blueprint、Aurora 或逐页色值说明复制样式。

## 3. 状态语义

### 3.1 语义级别

| 语义级别 | 含义 | Badge 变体 |
| --- | --- | --- |
| success | 已就绪、成功或有效 | success |
| warning | 仍可用但需关注，或等待用户介入 | warning |
| danger | 已出问题、失败或不可用 | danger |
| info | 排队、执行中或验证中的过程状态 | info |
| neutral | 主动终止、初始或无对象 | neutral |

### 3.2 状态映射

| 状态机 | wire 值 | 中文名 | 语义级别 |
| --- | --- | --- | --- |
| 证书 | pending_issue | 待签发 | info |
| 证书 | issuing | 签发中 | info |
| 证书 | issue_failed | 签发失败 | danger |
| 证书 | valid | 有效 | success |
| 证书 | expiring_soon | 即将到期 | warning |
| 证书 | renewing | 续签中 | info |
| 证书 | renewal_failed | 续签失败 | danger |
| 证书 | expired | 已过期 | danger |
| 证书 | revoking | 吊销中 | info |
| 证书 | revoked | 已吊销 | neutral |
| 任务 | queued | 排队 | info |
| 任务 | running | 执行中 | info |
| 任务 | succeeded | 成功 | success |
| 任务 | failed | 失败 | danger |
| 任务 | cancelled | 已取消 | neutral |
| 挑战 | pending | 待验证 | info |
| 挑战 | awaiting_manual | 等待手动配置 | warning |
| 挑战 | validating | 验证中 | info |
| 挑战 | passed | 验证通过 | success |
| 挑战 | failed | 验证失败 | danger |
| 挑战 | cancelled | 已取消 | neutral |
| 账户 | unconfigured | 未配置 | neutral |
| 账户 | registering | 注册中 | info |
| 账户 | registered | 已注册 | success |
| 账户 | registration_failed | 注册失败 | danger |
| 根 CA | active | 有效 | success |
| 根 CA | expired | 已过期 | danger |

状态语义与图标映射由 frontend/src/components/status-badge.tsx 统一呈现。排队和待开始状态用静态图标，执行状态用过程图标；不要在其他组件重复维护状态字面量到颜色的映射。

### 3.3 域名列表的证书状态投影

域名关联证书按最紧急状态展示：失败（已过期、签发失败、续签失败）优先于即将到期，即将到期优先于有效；没有证书时展示中性状态。投影实现位于 CertProjectionBadge。

### 3.4 Dashboard 告警级别

- 指标仅展示 PRD 定义的证书总数、即将到期数和失败数。
- 即将到期和失败计数大于零时才使用对应语义强调；零值保持中性。
- 待处理列表先展示已过期和签发/续签失败，再展示即将到期。
- 红点只表示 dashboard 的待处理数量，清空待办后清零。

### 3.5 分类标签

签发方式、通配符、任务触发方式和创建方式属于分类，使用中性标签。测试环境可用 warning 提醒其非生产属性；分类颜色不能取代生命周期状态颜色。

## 4. 字体与技术值

字体遵循 COHESION 接入说明和全局样式，不在页面加载额外字体。指纹、序列号、TXT 记录、webroot 路径、监听地址和 PEM 等机器值使用等宽字体；面向用户的文案使用系统正文样式。

## 5. 密度与响应式

密度根据任务与屏幕宽度调整，不能依赖固定页面宽度。窄屏数据表可在自身容器内滚动；输入和主操作须保留足够的触控区域，页面滚动内容不能被固定导航遮挡。Shell 断点和导航布局以 COHESION 接入说明与 AppShell 实现为准。

## 6. 表面与层级

卡片、对话框、菜单、Tooltip 和 Toast 使用 COHESION 对应的容器/浮层 Token 与组件阴影。长内容和浮层保持清晰、不透明的表面；不用含义不明的玻璃材质类名，也不以模糊效果降低文字对比度。

## 7. 组件交互

### 7.1 按钮

按钮变体按操作层级使用：一个主操作突出当前任务，次要操作使用中性样式，破坏性操作使用危险样式。图标按钮必须有可访问名称。

### 7.2 Badge

生命周期状态经 StatusBadge；普通分类经 CategoryBadges。没有对应状态时使用中性外观，不手搓状态颜色。

### 7.3 表格

表格列和操作不超出页面 PRD。保持标题、行内容和操作对齐；窄屏只允许表格容器局部横向滚动。

### 7.4 Dialog 与 AlertDialog

删除、吊销、移除账户和取消任务需要二次确认，确认文案写明结果。关闭弹窗后焦点回到触发控件；键盘操作保持可用。

### 7.5 表单

字段提供可见标签、就近说明和字段级错误。Placeholder 不能代替标签；提交期间展示忙碌状态并防止重复提交。

### 7.6 菜单、Select、Tooltip 与 Separator

优先使用现有 Radix 包装组件，保留键盘导航、焦点管理、选中语义和清晰浮层表面。不可用操作应说明原因。

### 7.7 Toast

Toast 反馈已完成的操作结果，不代替字段校验或长时间任务状态。任务进度与结果以 API 状态为准。

### 7.8 Alert

风险、需要用户介入的挑战和可恢复失败使用对应语义 Alert，并附可执行的下一步说明。

### 7.9 加载、空和错误状态

数据视图需要 loading、empty、error 和适用时的 disabled 状态。错误提供重试或返回入口；空状态只在业务定义允许时出现。

### 7.10 可复制技术值

可复制值用等宽字体；复制成功或失败均给出可理解的反馈。密钥内容不得因为复制功能而写入日志。

### 7.11 挑战过程

HTTP-01 与 DNS-01 是状态推进，不是百分比进度。展示挑战状态、等待用户完成的步骤和失败原因，不使用虚假的百分比进度条。

## 8. App Shell

桌面、服务器和移动 Web 共用一个前端与一套组件。导航路由、断点、运行模式标签和移动“更多”入口以 docs/design/cohesion-adoption.md 与 frontend/src/components/app-shell.tsx 为准。

红点只挂总览入口，计数语义与 dashboard 共享。仅桌面或仅服务器的功能通过运行模式显隐，不得引入第二套颜色或组件主题。

## 9. 图标与可访问性

使用 lucide-react，与现有组件保持统一图标风格。状态图标由 StatusBadge 管理；装饰图标从辅助技术树隐藏，独立图标操作提供 aria-label。颜色不是状态或错误的唯一信号。

## 10. 硬约束

- H1：复用项目 shadcn/Radix 组件约定和 data-slot；不恢复旧式 forwardRef 包装。
- H2：颜色使用 COHESION Token；状态映射只由 StatusBadge 管理。
- H3：数据视图覆盖加载、空、错误及适用的禁用状态。
- H4：证书、任务、账户和根 CA 操作严格按 PRD 状态表显示或禁用；禁用操作说明原因。
- H5：删除、吊销、移除账户和取消任务必须二次确认。
- H6：私钥导出和公网监听风险提供明确提示及确认。
- H7：键盘和屏幕阅读器可用；文字对比度符合 WCAG AA；状态同时由文字或图标表达。
- H8：技术值使用等宽字体；敏感材料不进入日志。
- H9：浅色、深色均读取主题 Token；运行时切换由 frontend/src/stores/ui.ts 管理，不依赖旧版 .dark 规范。
- H10：桌面、服务器共用视觉与组件；仅对运行模式相关内容做显隐。
- H11：时间使用统一时间工具；有效期保留绝对时刻，创建/更新时间使用相对展示并按需提供精确时间。
- H12：列表字段、指标、操作和批量能力不超出对应 PRD 范围。
