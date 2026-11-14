# 页面设计提示词 · 总览页(dashboard / overview)

> 类型: 工作底稿(仅 output 登记,不送审)· 端: app · 模块: dashboard · 撰写: designer
> 依据(approved,直接消费):
> - 页面 PRD `docs/prd/pages/app/dashboard/overview.md`(F1–F5、页面流转、验收 1–14)
> - API 契约 `docs/architecture/api/app/dashboard.md`(`DashboardOverview` / `PendingCertItem` DTO 字段;**原型不写路径**,仅据其定字段)
> - 逐科品牌、Token、Shell 与组件样式：`docs/design/cohesion-adoption.md`；业务状态与交互约束：`docs/design/systems/app.md` §3、§7、§8、§10。
> - 证书状态语义引用 systems §3.2;不复述状态定义。

---

## 1. 页面目标

autohttps 启动首屏。把散落各模块的证书状态**聚合成一屏**:多少证书、谁即将到期、谁出了问题,并以红点 / 待处理清单把「该处理的证书」推到眼前。**纯聚合视图 + 快速入口**,页内不含任何增删改(验收 13)。

---

## 2. 布局结构(承 §8 App Shell)

- **App Shell**:左侧导航与顶栏遵循 COHESION 接入说明及 AppShell 实现;**红点仅挂总览项**(systems §3.4)。活动项为「总览」。
- **内容区**:沿用 App 页面容器,按可用宽度响应布局,自上而下三区块——
  1. **概览三指标**(F1):3 列指标卡(`grid` 3 列,窄屏折 1 列)。
  2. **待处理清单**(F2/F4):区块标题 + 清单卡片(逐条行)。
  3. **常用操作入口**(F5):4 个快速入口(`grid` 2/4 列)。

---

## 3. 区块与字段

### 3.1 概览三指标(F1,唯一允许的统计卡片 · §10-H12)

三张指标卡使用现有卡片组件,清楚区分指标名称、数值和语义图标;口径引用 API `DashboardOverview.metrics`:

| 卡 | 字段 | 视觉(严格照 systems §3.4) |
| --- | --- | --- |
| 证书总数 | `metrics.totalCount` | **中性**:数字 `text-foreground`、标签 `text-muted-foreground`、图标 `ShieldCheck`(muted) |
| 即将到期数 | `metrics.expiringSoonCount` | `>0` → 数字 `text-warning` + 图标 `Clock text-warning`;`=0` → 退回中性 |
| 失败数 | `metrics.failedCount` | `>0` → 数字 `text-danger` + 图标 `TriangleAlert text-danger`;`=0` → 退回中性 |

> **规则**:强调色只在计数 `>0` 出现(健康系统=中性,不无谓报警)。**不放第 4 张卡、不放任何分布图表**(§10-H12)。

### 3.2 待处理清单(F2 · F4,数据来自 `pendingItems[]`,服务端已排序)

- 每行字段(引用 `PendingCertItem`):关联域名 `domains[]`、当前状态 `status`(→ StatusBadge,systems §3.2)、有效期 `notAfter` + `daysUntilExpiry`(相对时间 + Tooltip 绝对,§10-H11;绝对时间 mono §10-H8)。
- **排序**(systems §3.4):已过期 `expired` 居首 → 其余告警级(`issue_failed` / `renewal_failed`)→ 关注级(`expiring_soon`)。
- **行视觉分级**:告警级行使用 danger 语义,关注级行使用 warning 语义(systems §3.4);告警整体权重 > 关注。
- **跳转(F4)**:行主体点击 → 该证书详情(`certificateId`);失败态行额外给「失败原因 / 查看任务」`link` 按钮 → 最近任务(`latestTaskId`,为 null 时不出该入口)。
- **正向空态**(systems §7.9):当 `pendingCount=0` → `CheckCircle2 text-success` +「全部证书状态良好」,**无 CTA、无红点**。原型主渲染正常态;正向空态在提示词说明,可选演示。

### 3.3 红点(F3 · systems §3.4)

- 侧栏「总览」项展示 `pendingCount`(即将到期 + 失败);具体标记样式由 COHESION 接入组件决定,`>99` 显示 `99+`。
- **形态差异是页面级标注**(§10-H10):桌面=系统托盘角标(仅桌面);服务器=浏览器内(仅服务器);触发规则一致,原型以窗口内红点演示,附注标注形态差异。

### 3.4 常用操作入口(F5,前端路由,非 API)

4 个快速入口卡 / 按钮:发起签发(`Plus`)、查看全部证书(`ShieldCheck`)、域名管理(`Globe`)、任务与历史(`ListChecks`)。仅跳转,不在页内执行(验收 13)。「发起签发」为唯一主操作 → `default` 主色;其余 `outline` / 卡片式。

---

## 4. 交互与四态(§10-H3)

- **normal**(主渲染):三指标 + 5 条待处理 + 快速入口。
- **loading**:指标卡 / 清单行 `Skeleton`(形状贴合)。
- **empty(正向空)**:待处理清单为空 → systems §7.9 正向空态。
- **error**:聚合拉取失败 → 内容区 `destructive` Alert + 「重试」(systems §7.9)。
- 本页无破坏性操作、无表单(纯聚合);无 disabled 操作态。

---

## 5. 视觉 token 引用(不复制值)

品牌、色彩、字号、间距、圆角、边界与图标遵循 `docs/design/cohesion-adoption.md` 和现有组件；指标语义与待办排序遵循 systems §3.4。**颜色使用语义 Token，不写裸色值(systems §10-H2)**。

---

## 6. 硬约束核对点(本页相关)

- H2 无写死色值;状态经 StatusBadge + systems §3.2。
- H3 四态齐(normal 主渲染 + loading/empty(正向空)/error 说明或演示)。
- H8 有效期绝对时间 mono;H11 相对 + Tooltip 绝对。
- H10 红点形态差异为页面级标注,一套视觉。
- **H12 不越界**:仅三指标、无分布图表、无第 4 卡、无批量、无全局通知铃;页内无任何增删改。

---

## 7. 原型示例数据

- 指标:`totalCount=24`、`expiringSoonCount=2`、`failedCount=3` → `pendingCount=5`(演示失败染红 + 即将到期染 amber + 总数中性)。
- 待处理清单(已排序,5 条):
  1. `expired` · legacy.example.com · 已过期 7 天(失效 2026-07-09)
  2. `renewal_failed` · mail.example.com · 失效 2026-07-20 · 有失败原因入口
  3. `issue_failed` · new.internal.corp · 无有效期 · 有失败原因入口
  4. `expiring_soon` · api.example.com · 9 天后到期(失效 2026-07-25)
  5. `expiring_soon` · shop.example.com · 12 天后到期(失效 2026-07-28)
- 红点计数 5;当前日期基准 2026-07-16。

---

## 8. 边界(本页不含,PRD §2)

证书 / 任务 / 域名的任何增删改、证书状态判定 / 扫描(归 certificates)、多渠道通知、按签发方式 / CA 的分布统计。
