# 设计系统 · app 端(端级契约)

> 文档状态: draft(待 orchestrator 统一送审)· 层级: 端级设计契约 · 端点: app · 撰写: designer
> 信任基础(approved,直接消费不重新推导):
> - **TECH.md §1.4 / AR8**:前端栈已定——shadcn/ui(手写源码,遵 v4 / React 19 约定:`data-slot`、无 `forwardRef`、`@theme inline` OKLCH)+ Radix primitives + Tailwind CSS v4(`@theme` token、CSS-first、OKLCH)+ lucide-react 图标 + sonner。**designer 消费 shadcn + Tailwind v4,定 `@theme` 的 token 值**;前端架构层(react-query/zustand 分工、路由、表单、react-table、ts-rs 绑定)归基线,不在此立法。
> - **project.md**:跨平台(Win/Linux/macOS)桌面(Tauri)+ 服务器两形态**共享同一前端**;工具 / 管理型 UI,信息密度偏高;两形态差异靠运行模式 + 页面级「仅桌面 / 仅服务器」标注,**不拆端**(D1)。
> - **14 页面 PRD + 5 台状态机 + glossary(全部 approved)**:决定本系统需覆盖的 UI 模式与**状态色语义**。
> - **定位**:本文件定「长什么样 + 交互规范」,是全端 HTML 原型的 DAG 上游;改一次全端原型 stale。
> - **纪律**:一切视觉 token 定义在此;**原型禁写死色值**。不越界写 API 路径 / 数据结构(那是 PRD / API 契约)。token 值用 **OKLCH**。

## 章节

0. 设计原则
1. 色板(OKLCH · 明暗两套 · shadcn 变量 + 状态色 + sidebar + chart)
2. `@theme inline` token 映射
3. **状态色语义(域关键立法)**
4. 排版(字体族 · 字号阶梯)
5. 间距与密度尺度
6. 圆角 · 描边 · 阴影
7. 组件形态(shadcn 清单 + 用法规范)
8. App Shell 布局
9. 图标规范(lucide)
10. **硬约束(原型自检逐条核对依据)**
11. 决策记录

---

## 0. 设计原则

| # | 原则 | 落地含义 |
| --- | --- | --- |
| P1 | **状态优先** | 证书 / 任务 / 挑战 / 账户 / 根 CA 的**当前状态**是界面第一信息;状态色语义(§3)是全端最强约束,任何状态呈现必须走统一语义色。 |
| P2 | **信息密度偏高、克制** | 工具型 UI:紧凑尺度(§5)、以边框而非重阴影分区、色彩克制(仅状态 / 主操作用色,分类标签用中性)。避免「彩虹汤」。 |
| P3 | **诚实的过程与失败** | 在线操作(签发 / 续签 / 吊销 / 注册 / 验证)可失败、可长等待;必须呈现 loading / 等待人工 / 失败 / 可重试,不掩盖(呼应各 flow 的过渡态与告警态)。 |
| P4 | **破坏性与风险显式化** | 删除 / 吊销 / 移除 / 取消走二次确认;导出私钥、公网暴露走风险 Alert + 确认(project §7 / roles §3)。 |
| P5 | **两形态一套视觉** | 桌面 / 服务器共享同一 token 与组件;差异仅在页面级「仅桌面 / 仅服务器」显隐,**不产生第二套视觉 token**(D1)。 |
| P6 | **不越 PRD 边界加料** | 不主动加 PRD 未要求的统计卡片 / 图表 / 批量操作 / 列(§10 硬约束逐条锚定)。 |

---

## 1. 色板(OKLCH · 明暗两套)

> 落点:`frontend/src/index.css`(或等价全局样式)。**明暗两套**:`:root` = 亮色,`.dark` = 暗色(明暗切换机制见 §10-H7)。
> 命名遵 shadcn 约定;在其上**扩展**语义状态色(`--success/--warning/--danger/--info/--neutral` 各含 base / foreground / muted / muted-foreground 四元)。
> **「蓝图 Blueprint」基调**:亮面=暖白纸底(hue≈90 微暖,非纯白)+ 墨蓝前景;品牌主色=群青(hue≈268,高彩度,信任 / 基础设施 / HTTPS 语义);info 状态色=天青(hue≈222),与群青主色以 46° 色相差明确分野(DS12)。

### 1.1 亮色 `:root`

```css
:root {
  /* 基底与前景(暖纸底 + 墨蓝字) */
  --background: oklch(0.982 0.004 90);
  --foreground: oklch(0.235 0.02 265);
  --card: oklch(0.995 0.002 90);
  --card-foreground: oklch(0.235 0.02 265);
  --popover: oklch(0.995 0.002 90);
  --popover-foreground: oklch(0.235 0.02 265);

  /* 品牌主色·群青(主操作 / 焦点环) */
  --primary: oklch(0.50 0.20 268);
  --primary-foreground: oklch(0.97 0.005 265);

  /* 次级 / 柔和 / 强调(hover 底,微染群青) */
  --secondary: oklch(0.955 0.006 90);
  --secondary-foreground: oklch(0.32 0.02 265);
  --muted: oklch(0.96 0.005 90);
  --muted-foreground: oklch(0.53 0.02 265);
  --accent: oklch(0.94 0.01 268);
  --accent-foreground: oklch(0.32 0.02 265);

  /* 破坏性(= 危险语义主色) */
  --destructive: oklch(0.585 0.20 25);
  --destructive-foreground: oklch(0.985 0.003 90);

  /* 描边 / 输入框 / 焦点环(暖灰发丝线) */
  --border: oklch(0.912 0.006 90);
  --input: oklch(0.912 0.006 90);
  --ring: oklch(0.50 0.20 268);

  /* —— 语义状态色(§3 立法用)—— */
  --success: oklch(0.60 0.14 152);
  --success-foreground: oklch(0.99 0 0);
  --success-muted: oklch(0.945 0.04 152);
  --success-muted-foreground: oklch(0.44 0.10 152);

  --warning: oklch(0.70 0.14 75);
  --warning-foreground: oklch(0.27 0.05 75);
  --warning-muted: oklch(0.955 0.05 80);
  --warning-muted-foreground: oklch(0.48 0.09 70);

  --danger: oklch(0.585 0.20 25);
  --danger-foreground: oklch(0.985 0.003 90);
  --danger-muted: oklch(0.945 0.035 25);
  --danger-muted-foreground: oklch(0.50 0.17 25);

  --info: oklch(0.58 0.13 222);
  --info-foreground: oklch(0.99 0 0);
  --info-muted: oklch(0.945 0.035 222);
  --info-muted-foreground: oklch(0.44 0.11 222);

  --neutral: oklch(0.54 0.02 265);
  --neutral-foreground: oklch(0.99 0 0);
  --neutral-muted: oklch(0.955 0.005 265);
  --neutral-muted-foreground: oklch(0.45 0.015 265);

  /* 红点 / 通知(= danger,§8 红点立法引用) */
  --notification: oklch(0.585 0.20 25);
  --notification-foreground: oklch(0.985 0.003 90);

  /* chart(shadcn 约定;MVP 预留,dashboard 无分布图表,见 §10-H12) */
  --chart-1: oklch(0.50 0.20 268);
  --chart-2: oklch(0.60 0.14 152);
  --chart-3: oklch(0.70 0.14 75);
  --chart-4: oklch(0.585 0.20 25);
  --chart-5: oklch(0.54 0.02 265);

  /* sidebar(App Shell 侧栏,§8) */
  --sidebar: oklch(0.965 0.005 90);
  --sidebar-foreground: oklch(0.235 0.02 265);
  --sidebar-primary: oklch(0.50 0.20 268);
  --sidebar-primary-foreground: oklch(0.97 0.005 265);
  --sidebar-accent: oklch(0.94 0.008 265);
  --sidebar-accent-foreground: oklch(0.235 0.02 265);
  --sidebar-border: oklch(0.912 0.006 90);
  --sidebar-ring: oklch(0.50 0.20 268);

  /* 圆角基准(§6) */
  --radius: 0.375rem;
}
```

### 1.2 暗色 `.dark`

> 暗色底取深墨蓝(oklch(0.16 … hue≈265)),卡片较底色微抬做层次;描边用半透明白(shadcn v4 暗色惯例)。状态色的 muted 反转为「低亮度高饱和底 + 高亮文字」。**暗色下主色 / 状态色的前景色取深色墨**(高亮底 + 深字对比度优于白字,达 AA)。

```css
.dark {
  --background: oklch(0.16 0.012 265);
  --foreground: oklch(0.945 0.008 265);
  --card: oklch(0.19 0.014 265);
  --card-foreground: oklch(0.945 0.008 265);
  --popover: oklch(0.19 0.014 265);
  --popover-foreground: oklch(0.945 0.008 265);

  --primary: oklch(0.68 0.16 268);
  --primary-foreground: oklch(0.17 0.02 268);

  --secondary: oklch(0.24 0.014 265);
  --secondary-foreground: oklch(0.945 0.008 265);
  --muted: oklch(0.24 0.014 265);
  --muted-foreground: oklch(0.68 0.02 265);
  --accent: oklch(0.27 0.02 268);
  --accent-foreground: oklch(0.945 0.008 265);

  --destructive: oklch(0.66 0.19 25);
  --destructive-foreground: oklch(0.17 0.03 25);

  --border: oklch(1 0 0 / 9%);
  --input: oklch(1 0 0 / 13%);
  --ring: oklch(0.68 0.16 268);

  --success: oklch(0.70 0.14 152);
  --success-foreground: oklch(0.17 0.03 152);
  --success-muted: oklch(0.27 0.05 152);
  --success-muted-foreground: oklch(0.82 0.11 152);

  --warning: oklch(0.78 0.14 78);
  --warning-foreground: oklch(0.20 0.04 78);
  --warning-muted: oklch(0.29 0.05 75);
  --warning-muted-foreground: oklch(0.86 0.11 80);

  --danger: oklch(0.66 0.19 25);
  --danger-foreground: oklch(0.17 0.03 25);
  --danger-muted: oklch(0.29 0.07 25);
  --danger-muted-foreground: oklch(0.82 0.12 25);

  --info: oklch(0.68 0.12 222);
  --info-foreground: oklch(0.16 0.03 222);
  --info-muted: oklch(0.27 0.05 222);
  --info-muted-foreground: oklch(0.82 0.10 222);

  --neutral: oklch(0.70 0.015 265);
  --neutral-foreground: oklch(0.17 0.01 265);
  --neutral-muted: oklch(0.24 0.012 265);
  --neutral-muted-foreground: oklch(0.76 0.015 265);

  --notification: oklch(0.66 0.19 25);
  --notification-foreground: oklch(0.17 0.03 25);

  --chart-1: oklch(0.68 0.16 268);
  --chart-2: oklch(0.70 0.14 152);
  --chart-3: oklch(0.78 0.14 78);
  --chart-4: oklch(0.66 0.19 25);
  --chart-5: oklch(0.70 0.015 265);

  --sidebar: oklch(0.185 0.013 265);
  --sidebar-foreground: oklch(0.945 0.008 265);
  --sidebar-primary: oklch(0.68 0.16 268);
  --sidebar-primary-foreground: oklch(0.17 0.02 268);
  --sidebar-accent: oklch(0.235 0.014 265);
  --sidebar-accent-foreground: oklch(0.945 0.008 265);
  --sidebar-border: oklch(1 0 0 / 9%);
  --sidebar-ring: oklch(0.68 0.16 268);
}
```

---

## 2. `@theme inline` token 映射

> Tailwind v4 CSS-first:把上面的 CSS 变量映射为 `--color-*` 主题 token,`bg-* / text-* / border-*` 等工具类据此生成。**原型只用这些工具类或 `var(--…)`,禁裸色值(§10-H1)。**

```css
@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-destructive-foreground: var(--destructive-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);

  /* 语义状态色 → 生成 bg-success / text-success-muted-foreground 等 */
  --color-success: var(--success);
  --color-success-foreground: var(--success-foreground);
  --color-success-muted: var(--success-muted);
  --color-success-muted-foreground: var(--success-muted-foreground);
  --color-warning: var(--warning);
  --color-warning-foreground: var(--warning-foreground);
  --color-warning-muted: var(--warning-muted);
  --color-warning-muted-foreground: var(--warning-muted-foreground);
  --color-danger: var(--danger);
  --color-danger-foreground: var(--danger-foreground);
  --color-danger-muted: var(--danger-muted);
  --color-danger-muted-foreground: var(--danger-muted-foreground);
  --color-info: var(--info);
  --color-info-foreground: var(--info-foreground);
  --color-info-muted: var(--info-muted);
  --color-info-muted-foreground: var(--info-muted-foreground);
  --color-neutral: var(--neutral);
  --color-neutral-foreground: var(--neutral-foreground);
  --color-neutral-muted: var(--neutral-muted);
  --color-neutral-muted-foreground: var(--neutral-muted-foreground);
  --color-notification: var(--notification);
  --color-notification-foreground: var(--notification-foreground);

  --color-chart-1: var(--chart-1);
  --color-chart-2: var(--chart-2);
  --color-chart-3: var(--chart-3);
  --color-chart-4: var(--chart-4);
  --color-chart-5: var(--chart-5);

  --color-sidebar: var(--sidebar);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-ring: var(--sidebar-ring);

  /* 圆角阶梯(shadcn 惯例,承 --radius) */
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);

  /* 字体族(§4) */
  --font-sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto,
    "Helvetica Neue", Arial, "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif;
  --font-mono: ui-monospace, "SFMono-Regular", "JetBrains Mono", "Cascadia Code",
    Consolas, "Liberation Mono", Menlo, monospace;
}
```

> `.dark` 挂在 `<html>` 或根容器上即整树切换(§10-H7)。HTML 原型静态演示两套时,给根节点加 / 去 `class="dark"` 即可。

---

## 3. 状态色语义(域关键 · 全端最强立法)

> **这是本设计系统的核心契约**。5 台状态机(证书 10 态 / 任务 5 态 / 挑战 6 态 / 账户 4 态 / 根 CA 2 态)的每个态,统一映射到 5 个语义级与对应 Badge 变体、图标。**任何状态呈现必须走此表,禁自定义色**(§10-H2)。状态**中文名**照各 flow(引用不复述定义)。

### 3.1 五语义级 → token / Badge 变体 / 图标

| 语义级 | 含义 | Badge 变体 | 软底 / 文字 token | 主色 token(点 / 图标 / 强调) |
| --- | --- | --- | --- | --- |
| **success 稳态·成功** | 已就绪 / 成功 / 有效 | `success` | `bg-success-muted` / `text-success-muted-foreground` | `text-success` |
| **warning 关注·等待** | 仍可用但需关注 / 等待人工 | `warning` | `bg-warning-muted` / `text-warning-muted-foreground` | `text-warning` |
| **danger 告警·失败** | 已出问题 / 已不可用 / 失败 | `danger` | `bg-danger-muted` / `text-danger-muted-foreground` | `text-danger` |
| **info 进行中** | 排队 / 执行中 / 验证中(过程态) | `info` | `bg-info-muted` / `text-info-muted-foreground` | `text-info` |
| **neutral 中性·终态** | 主动终止 / 概念初始 / 无对象 | `neutral` | `bg-neutral-muted` / `text-neutral-muted-foreground` | `text-neutral-muted-foreground` |

> Badge 默认取**软底样式**(tinted bg + 饱和文字),密集表格里最可读;`h-5 text-xs px-2 rounded-md`,可选前置 6px 状态圆点或 12px lucide 图标。

### 3.2 全状态映射表(逐态立法)

| 状态机 | 状态(wire 值) | 中文名 | 语义级 | 图标(lucide) | 图标动效 |
| --- | --- | --- | --- | --- | --- |
| 证书 | `pending_issue` | 待签发 | info | `Clock` | 静态 |
| 证书 | `issuing` | 签发中 | info | `Loader2` | 旋转 |
| 证书 | `issue_failed` | 签发失败 | danger | `TriangleAlert` | 静态 |
| 证书 | `valid` | 有效 | success | `CheckCircle2` | 静态 |
| 证书 | `expiring_soon` | 即将到期 | warning | `Clock` | 静态 |
| 证书 | `renewing` | 续签中 | info | `Loader2` | 旋转 |
| 证书 | `renewal_failed` | 续签失败 | danger | `TriangleAlert` | 静态 |
| 证书 | `expired` | 已过期 | danger | `ShieldAlert` | 静态 |
| 证书 | `revoking` | 吊销中 | info | `Loader2` | 旋转 |
| 证书 | `revoked` | 已吊销 | neutral | `Ban` | 静态 |
| 任务 | `queued` | 排队 | info | `Clock` | 静态 |
| 任务 | `running` | 执行中 | info | `Loader2` | 旋转 |
| 任务 | `succeeded` | 成功 | success | `CheckCircle2` | 静态 |
| 任务 | `failed` | 失败 | danger | `CircleX` | 静态 |
| 任务 | `cancelled` | 已取消 | neutral | `CircleSlash` | 静态 |
| 挑战 | `pending` | 待验证 | info | `Clock` | 静态 |
| 挑战 | `awaiting_manual` | 等待手动配置 | **warning** | `Hourglass` | 静态 |
| 挑战 | `validating` | 验证中 | info | `Loader2` | 旋转 |
| 挑战 | `passed` | 验证通过 | success | `CheckCircle2` | 静态 |
| 挑战 | `failed` | 验证失败 | danger | `CircleX` | 静态 |
| 挑战 | `cancelled` | 已取消 | neutral | `CircleSlash` | 静态 |
| 账户 | `unconfigured` | 未配置 | neutral | `CircleDashed` | 静态 |
| 账户 | `registering` | 注册中 | info | `Loader2` | 旋转 |
| 账户 | `registered` | 已注册 | success | `CheckCircle2` | 静态 |
| 账户 | `registration_failed` | 注册失败 | danger | `CircleX` | 静态 |
| 根 CA | `active` | 有效 | success | `ShieldCheck` | 静态 |
| 根 CA | `expired` | 已过期 | danger | `ShieldAlert` | 静态 |

**立法要点**:
- **info 内两类图标区分**:排队 / 待开始(`pending_issue` / `queued` / `pending`)用**静态时钟** `Clock`;实际执行中(`issuing` / `renewing` / `revoking` / `running` / `validating` / `registering`)用**旋转 spinner** `Loader2 animate-spin`——同天青(info)语义,图标区分「等待被处理」与「正在处理」。
- **`awaiting_manual` 归 warning 非 info**:它虽处 flow「进行中」,但语义是「等你去加 TXT」的**待处理**,用 amber + `Hourglass` 与 dashboard 待处理级别一致(flows/acme §3.1 / DA2)。
- **`revoking`(info 天青)vs `revoked`(neutral 灰)**:进行中态与主动终态分色。

### 3.3 域名列表「证书态投影」映射(domains/list F1)

> 域名按其关联证书**最紧急**态投影(flows/domains §2.1;优先级 失败 > 即将到期 > 有效):

| 投影 | 语义级 | Badge 变体 | 中文 |
| --- | --- | --- | --- |
| 失败(已过期 / 签发失败 / 续签失败) | danger | `danger` | 失败 |
| 即将到期 | warning | `warning` | 即将到期 |
| 有效 | success | `success` | 有效 |
| 无证书 | neutral | `neutral`(outline 更弱) | 无证书 |

### 3.4 dashboard 告警级 > 关注级 视觉权重(立法)

> flows/dashboard §3.1:告警级(已过期 / 签发失败 / 续签失败)权重 > 关注级(即将到期),已过期居首。视觉据此:

- **三指标卡**(dashboard 唯一统计卡片,§10-H12):
  - `证书总数`:**中性**——数字 `text-foreground`、标签 `text-muted-foreground`、图标 `ShieldCheck` muted。
  - `即将到期数`:**关注级**——`>0` 时数字 `text-warning`、图标 `Clock text-warning`;`=0` 时退回中性。
  - `失败数`:**告警级**——`>0` 时数字 `text-danger`、图标 `TriangleAlert text-danger`;`=0` 时退回中性。
  - **规则**:强调色只在 `计数>0` 时出现(健康系统=中性,不无谓报警)。
- **待处理清单**(F2):
  - 排序:`已过期` → `签发失败` / `续签失败`(告警级)→ `即将到期`(关注级);已过期居首。
  - 行视觉:告警级行左侧 `3px` `bg-danger` 指示条 + `danger` Badge;关注级行左侧 `3px` `bg-warning` 指示条 + `warning` Badge。告警级整体权重强于关注级。
  - 每条显示:关联域名、当前状态 Badge、有效期。
- **红点计数** = 即将到期数 + 失败数;计数 `>0` 亮红点(§8 红点立法)。全部处置完 → 触发集空 → 红点清零 → 待处理清单进「正向空状态」(§7 Empty「全部证书状态良好」,不出 CTA)。

### 3.5 分类标签(**不用**状态色,防彩虹汤)

> 以下是**分类**而非**生命周期状态**,一律用**中性 outline / secondary** Badge,不占用语义色:

| 标签 | 取值 | 呈现 |
| --- | --- | --- |
| 签发方式 | ACME / 自签 | `outline` 中性 Badge(可配 `KeyRound` / `Landmark` 前缀图标) |
| 通配符 | 通配符域名 | `outline` 中性小 Badge「通配符」 |
| 触发方式 | 手动 / 自动 / 清理 | `outline` 中性 Badge |
| CA 环境 | 生产 / 测试 | 生产=中性 outline;测试=`warning` outline(弱提示非生产) |
| 创建方式 | 新建 / 导入 | `outline` 中性 Badge |

---

## 4. 排版

### 4.1 字体族

| token | 用途 | 说明 |
| --- | --- | --- |
| `font-sans`(`--font-sans`) | 全局默认 | 系统 UI 栈 + CJK 回退(产品 UI 为中文) |
| `font-mono`(`--font-mono`) | **技术值** | 指纹 / 序列号 / TXT 记录名值 / webroot 路径 / 监听地址端口 / 证书链 PEM / 执行日志——凡「可复制、需对齐、机器味」的值一律 mono(§10-H8) |

### 4.2 字号阶梯(工具型 · 紧凑;工作主力为 14px)

| 语义 | Tailwind | px / line-height | 字重 | 用途 |
| --- | --- | --- | --- | --- |
| page-title | `text-xl` | 20 / 28 | 600 | 页面主标题 |
| section-title | `text-base` | 16 / 24 | 600 | 区块 / 卡片组标题 |
| card-title | `text-sm` | 14 / 20 | 600 | 卡片标题、指标卡标签 |
| **body**(默认) | `text-sm` | 14 / 20 | 400 | 正文、表格单元格、详情值——**全局默认字号** |
| body-strong | `text-sm` | 14 / 20 | 500–600 | 强调值、关键字段 |
| label | `text-[13px]` | 13 / 18 | 500 | 表单标签、表头 |
| meta / caption | `text-xs` | 12 / 16 | 400 | 时间戳、次要说明、Badge、面包屑 |
| metric-number | `text-2xl`~`text-3xl` | 24–30 | 600–700 | dashboard 三指标数字 |
| mono-inline | `text-[13px] font-mono` | 13 | 400 | 行内技术值 |

> 全局 `body` 设 `text-sm`(14px)是工具 / 管理类 UI 的密度惯例;`text-base`(16px)仅用于确需舒展的正文段落,列表 / 表格 / 表单一律 14px。

---

## 5. 间距与密度尺度

> 基准单位 4px(Tailwind `0.25rem`)。工具型 UI 取紧凑档。

### 5.1 间距阶梯(常用步)

| token | px | 典型用途 |
| --- | --- | --- |
| `1` | 4 | 图标与文字间隙、Badge 内元素 |
| `1.5` | 6 | 标签与输入框、状态点与文字 |
| `2` | 8 | 紧凑内间距、表格单元格纵向 |
| `3` | 12 | 表格单元格横向、按钮组间隙 |
| `4` | 16 | 表单字段纵向间距、卡片内元素 |
| `6` | 24 | 卡片内边距、页面内容内边距、区块间距 |
| `8` | 32 | 大区块 / 页首与内容 |

### 5.2 组件密度规格(立法基线)

| 元素 | 规格 |
| --- | --- |
| 侧栏宽 | 展开 `240px`;图标轨 `56px` |
| 顶栏高 | `56px`(`h-14`) |
| 页面内容内边距 | 桌面宽 `p-6`(24);窄屏 `p-4`(16) |
| 详情 / 表单可读宽 | 表单 `max-w-3xl`(≈768);详情可两栏,单列内容块 `max-w-3xl` |
| 列表页 | 表格**满幅**(不强行限宽) |
| 卡片内边距 | 内容卡 `p-6`;指标卡 `p-5` |
| 表格行高 | 数据行 `h-11`(44)、`px-3 py-2.5`;表头 `h-10`(40) `text-[13px]` muted;可选紧凑档 `h-9`(36) |
| 表单字段 | 组间 `space-y-4`;label→控件 `space-y-1.5` |
| 按钮高 | `sm=h-8`(32)、`default=h-9`(36)、`lg=h-10`(40)、`icon=size-9`;工具栏 / 表格内用 `sm` |
| 输入框高 | `h-9`(36) |
| Badge 高 | `h-5`(20)`text-xs px-2` |
| 对话框宽 | 确认类 `max-w-md`(≈448);表单类 `max-w-lg`~`max-w-xl` |

---

## 6. 圆角 · 描边 · 阴影

- **圆角**:`--radius: 0.375rem`(6px),锐利克制显「精密仪器感」。阶梯 `sm/md/lg/xl`(§2)。Badge / Input / Button 用 `md`~`lg`;Card / Dialog 用 `lg`~`xl`;圆点 / 头像用 `rounded-full`。
- **描边**:结构主要靠 `border-border` 分区(暗色为半透明白),**边框优先于阴影**(P2 扁平精确)。表格、卡片、输入框统一 `1px` 边框。
- **阴影**:克制。Card 静置可无阴影或 `shadow-xs`;浮层(Dropdown / Popover / Dialog / Toast)用 `shadow-md`~`shadow-lg` 建立层级;避免大面积重投影。

---

## 7. 组件形态(shadcn 清单 + 用法规范)

> 全部手写于 `frontend/src/components/ui/`,遵 v4 / React 19 约定(§10-H1)。下表锚定「用哪些组件、各自规范」;原型只用这些组件表达对应模式。

### 7.1 Button(变体 → 操作语义)

| 变体 | 语义 | 用在哪(示例) |
| --- | --- | --- |
| `default`(主色实心) | **每个视图唯一主操作** | 发起签发确认 / 保存设置 / 确认注册账户 / 创建根 CA / 确认续签 |
| `secondary` | 非破坏性次操作 | 编辑、非破坏性「取消」 |
| `outline` | 工具栏 / 表格内中性操作 | 导出、筛选触发、刷新 |
| `ghost` | 低强调 / 图标按钮 / 行操作触发 | 表格行 `MoreHorizontal`、侧栏项 |
| `destructive`(危险实心) | **仅**破坏性操作的 AlertDialog **最终确认**按钮 | 删除 / 吊销 / 移除账户 / 取消任务的确认 |
| `link` | 行内跳转 | 跳关联任务 / 跳证书详情 / 跳域名详情 |

- **两步破坏性范式**:行内触发按钮用 `ghost`/`outline` + `text-danger`(如「删除」),点击弹 `AlertDialog`,其内最终确认按钮才用 `destructive` 实心。
- 图标按钮必须 `aria-label` + `Tooltip`(§10-H6)。
- 异步操作按钮:pending 时 `disabled` 且内嵌 `Loader2 animate-spin`。

### 7.2 Badge

- **状态 Badge**:扩展 cva 增加 `success | warning | danger | info | neutral` 五变体(软底样式,取 §3.1 token)。**原型渲染状态一律经统一 `StatusBadge`**(入参=状态 wire 值 → 查 §3.2 得中文名 + 变体 + 图标),不手搓色值 / 不手写映射(§10-H2)。
- **分类 Badge**:签发方式 / 通配符 / 触发方式 / CA 环境 / 创建方式用中性 `outline`(§3.5)。

### 7.3 Table / DataTable(@tanstack/react-table)

- 用途:证书列表 / 域名列表 / 任务列表 / ACME 账户列表 / 根 CA 列表 / 根 CA 详情内网证书概览。
- **列 = PRD F 清单所列字段,禁自增列**(§10-H12)。状态列用 `StatusBadge`;时间列 mono + 相对时间(§10-H11)。
- **筛选**:置于表格上方**工具栏**(状态 `Select` / 签发方式 `Select` / 关键字 `Input`(前置 `Search` 图标) / 时间范围);组合生效(各列表 PRD 均要求可组合)。任务列表「队列 = 进行中态筛选」用**分段控件 / Tabs 作筛选器**(非独立页,flows/tasks DEC1)。
- **行操作**:≤2 个用行尾内联图标按钮;≥3 或含破坏性用 `DropdownMenu`(`MoreHorizontal`)。行主体点击进详情(操作控件区域阻止冒泡)。
- **禁批量**:所有列表 PRD 明示无批量——**不放行选择框 / 全选 / 批量操作条**(§10-H12)。
- 长 hostname `truncate` + `Tooltip` 全量;表头 sticky;底部 `Pagination`(page/pageSize,TECH §3.3)。
- 三态齐备:loading=骨架行;empty=Empty 组件;error=行区 Alert + 重试(§10-H3)。

### 7.4 Dialog / AlertDialog / Sheet

| 组件 | 用途 |
| --- | --- |
| `AlertDialog` | **破坏性二次确认**:删除证书 / 删除域名 / 吊销证书 / 移除账户 / 取消任务。须写清后果;确认按钮 `destructive`。 |
| `Dialog` | 非破坏性表单 / 面板:新增域名、编辑域名、导出内容选择面板、(可选)配置 ACME 账户。 |
| `Sheet` | 窄屏侧栏抽屉(§8 响应式);必要时右侧详情抽屉。 |

- **高风险追加确认**:导出私钥、监听地址设为对外可达——`AlertDialog` 内嵌 `danger` Alert(§7.8)+ **勾选确认框**后方可提交(project §7 / roles §3)。
- 删除被拦截(域名仍被证书关联 / 证书进行中态不可删)→ 不弹确认,直接 `Tooltip`/内联提示禁用原因并给跳转入口。

### 7.5 Form(react-hook-form + zod)+ 控件

- 结构:shadcn `Form`(`FormField/FormItem/FormLabel/FormControl/FormMessage`);校验经 zod;字段错误 `FormMessage` 行内 `text-danger text-xs`;必填项标记;提交中禁用并 spinner。
- 控件选型:
  - `Select`:枚举单选(目标 CA、签发方式细项、默认 ACME 账户、验证方式类别)。
  - `RadioGroup` / 分段:二选一大分支(签发方式 ACME/自签、新增根 CA 创建/导入)。
  - `Switch`:开关(自动续签、开机自启)。
  - `Input`:hostname、邮箱、续签提前天数(number)、webroot 路径、监听地址 / 端口、根 CA 名称 / 有效期。
  - `Input[type=password]`:导入根 CA 私钥解密口令。
  - `Textarea`:域名备注。
  - `Checkbox`:同意服务条款(未勾选禁用注册,acme/accounts 验收4);高风险确认勾选。
- 校验错误走**行内**,不用 toast;成功 / 服务端失败走 toast(§7.7)。

### 7.6 DropdownMenu · Tabs · Tooltip · Separator · Breadcrumb · Pagination

- `DropdownMenu`:行操作溢出菜单;项可按状态 `disabled` + `Tooltip` 原因;破坏性项 `text-danger`。
- `Tabs` / 分段控件:**仅**用作同页视图切换 / 筛选(任务队列-历史筛选)。**设置页不用 Tabs**——PRD 要求单页分区(settings DEC1),用带标题的 `Card` 分区。
- `Tooltip`:图标按钮释义、禁用原因、截断全量、技术值全形。
- `Breadcrumb`:详情页层级(列表 / 详情);顶栏承载。
- `Pagination`:列表底部(任务历史只增,必备)。

### 7.7 Sonner(toast)

- 异步动作反馈:`success`(签发已发起 / 续签成功 / 账户注册成功 / 导出完成 / 保存成功)、`error`(吊销失败 / 注册失败等)、`info`/`warning`(如 DNS-01 校验通过后「可移除该 TXT」提示)。
- 颜色映射 §3 语义色;位置桌面右下。**不承载表单校验错误**(那走行内)。

### 7.8 Alert(风险 / 提示,非阻塞)

> shadcn 仅带 `default`/`destructive`,**扩展 `warning` 变体**(取 `--warning` 系):

| 变体 | 用途 |
| --- | --- |
| `destructive`(danger) | 导出私钥风险、监听地址公网暴露风险 |
| `warning`(amber) | 已过期根 CA 提示「不可再签发、需创建 / 导入新根接替」、移除账户「有证书正引用」影响提示、DNS-01「等待手动配置」引导 |
| `default`(info/中性) | 一般说明 / 空数据引导补充 |

### 7.9 Skeleton · Empty · 错误态

- `Skeleton`:形状贴合内容(表格骨架行、卡片骨架、详情字段骨架)。
- **Empty 组件**:`lucide` 图标(muted)+ 标题 + 描述 + 主 CTA。三种语气:
  1. **尚无对象**(引导创建):无域名 / 无证书 / 无账户(未配置)/ 无根 CA / 无任务——CTA 指向新建 / 配置。
  2. **筛选无命中**:提示 + 「清除筛选」次 CTA。
  3. **正向空**(dashboard 待处理清单为空):`CheckCircle2 text-success` +「全部证书状态良好」,**无 CTA、无红点**。
- **错误态**:react-query error → 内容区 `destructive` Alert + 「重试」按钮;页面级错误边界兜底(§10-H3)。

### 7.10 复用范式:可复制技术值块

> TXT 记录(名 + 值)、指纹 / 序列号、webroot 路径等反复出现——立法统一范式:
> `font-mono` 只读 `Input` 或代码块 + 行尾 `Copy` 图标按钮(点后 `Copy→Check` 反馈 + toast「已复制」)+ `Tooltip` 全形。DNS-01 的 TXT 记录名、值各一块,强调「便于复制」(acme/challenge-wizard F4)。

### 7.11 复用范式:挑战进度(不用假进度条)

> HTTP-01 自动进度 / DNS-01 手动流程无百分比语义——**禁进度条**。用**竖向步骤 / 时间线**呈现挑战状态机(待验证→验证中→通过 / 失败;DNS-01 多一段 等待手动配置),每步一个 §3.2 状态图标 + 文案;多域名(SAN)各一行挑战,可各处不同态(整体判定 flows/acme §3.4)。DNS-01 停在「等待手动配置」时展示 TXT 复制块(§7.10)+ 主按钮「确认已添加」+ 可选本地预检结果。

---

## 8. App Shell 布局

### 8.1 结构

```
┌───────────┬────────────────────────────────────────┐
│  Sidebar  │  Topbar(h-14):面包屑 / 页标题 · 主题切换  │
│  (240px)  ├────────────────────────────────────────┤
│  7 nav    │                                        │
│  ● 红点   │  Content(p-6,按页型限宽)               │
│           │                                        │
│  ─────    │                                        │
│  运行形态  │                                        │
│  主题切换  │                                        │
└───────────┴────────────────────────────────────────┘
```

- **Sidebar**(shadcn `Sidebar`,可折叠图标轨):顶部品牌;中部 7 导航项(图标 + 标签);底部**运行形态只读 chip**(桌面 / 服务器,取自 API,TECH §3.6)+ **主题切换**。活动项:`bg-sidebar-accent` + 主色左指示条 / 主色图标。
- **7 导航项**(顺序与图标立法,§9.1):总览 / 域名 / 证书 / ACME / 根 CA / 任务 / 设置。
- **红点**:仅挂在**总览**项(dashboard 是红点唯一落点,MVP 无独立通知中心)。
- **Topbar**:左=面包屑 / 页标题;右=主题切换(与侧栏底部二选一放置,勿重复)。**禁放全局通知铃 / 通知中心**(MVP 无多渠道通知,project §6.2 / §10-H12)。
- **Content**:滚动区;列表页满幅,详情 / 表单页限宽(§5.2)。

### 8.2 红点组件立法

- **点**:`8px` `rounded-full bg-notification`,置总览图标右上。
- **计数徽标**:`min-w-4 h-4 px-1 text-[10px] leading-none rounded-full bg-notification text-notification-foreground`,数值=待处理计数(§3.4),`>99` 显示 `99+`。
- **图标轨折叠**时:点 / 计数叠在图标角。
- **形态差异是页面级标注,非视觉 token 分叉**:桌面=系统托盘图标角标(仅桌面);服务器=浏览器内 dashboard(可选浏览器标签标题标记,仅服务器)。**红点触发规则两形态一致**(flows/dashboard §3.3);托盘角标 / 浏览器标记的载体差异在页面 PRD 以「仅桌面 / 仅服务器」标注,本设计系统只定义红点 / 计数徽标本身的样式。

### 8.3 响应式

| 断点 | 侧栏 | 表格 |
| --- | --- | --- |
| ≥1024 | 展开 240px | 满列 |
| 768–1024 | 图标轨 56px（悬浮 / 点击展开） | 次要列可隐 / 横向滚动 |
| <768 | 收进 `Sheet` 抽屉,顶栏出汉堡按钮 | 优先列 + 横向滚动 |

---

## 9. 图标规范(lucide-react)

- 统一 `lucide-react`;默认 `size-4`(16)行内 / 按钮内,`size-5`(20)导航,状态 Badge 内 `size-3`(12)。描边宽度用 lucide 默认。图标语义**不脱离** §3.2 / §9.1 立法,禁随意换图标表达同一状态。

### 9.1 导航图标(7 模块,立法)

| 模块 | lucide |
| --- | --- |
| 总览 dashboard | `LayoutDashboard` |
| 域名 domains | `Globe` |
| 证书 certificates | `ShieldCheck` |
| ACME acme | `BadgeCheck` |
| 根 CA local-ca | `Landmark` |
| 任务 tasks | `ListChecks` |
| 设置 settings | `Settings` |

### 9.2 操作图标(常用)

| 动作 | lucide |
| --- | --- |
| 发起签发 / 新增 | `Plus` |
| 续签 / 重试 | `RotateCw` |
| 吊销 | `Ban` |
| 删除 | `Trash2` |
| 导出 | `Download` |
| 复制 | `Copy` →(成功)`Check` |
| 跳转 / 外链 | `ExternalLink` / `ArrowUpRight` |
| 行溢出菜单 | `MoreHorizontal` |
| 筛选 / 搜索 | `Search` / `ListFilter` |
| 取消(任务) | `XCircle` |

> 状态图标见 §3.2(逐态);状态与操作图标是两套,勿混用。

---

## 10. 硬约束(原型自检逐条核对依据)

> **生成 HTML 原型后,逐条核对本章**。这是 prompt 之外的端级立法;违反即原型 stale / 打回。

- **H1 · shadcn v4 / React 19 规范**:组件根 `data-slot` 属性;**无 `forwardRef`**;一切视觉 token 来自 `@theme`(§1/§2)。
- **H2 · 禁写死色值**:原型**不得**出现裸 hex / rgb / 具体 oklch 字面量;颜色只走 Tailwind 主题工具类(`bg-* text-* border-*`)或 `var(--…)`。状态呈现必须经 `StatusBadge` + §3 映射,**禁手搓状态色 / 手写状态映射**。
- **H3 · 交互四态必备**:每个数据视图都要有 **loading(Skeleton)/ empty(Empty 三语气)/ error(react-query 边界 → Alert + 重试)/ disabled**;缺一即不合格。
- **H4 · 操作按状态禁用**:证书 / 任务 / 账户 / 根 CA 的生命周期操作按**当前状态**动态可用 / 禁用,严格照各页 PRD 的状态-操作表(如:证书详情 F2–F8 按 §2.3 转移;吊销**不含已过期**;进行中态不可删除;导出仅本地有文件态;任务重试仅 `failed`、取消仅 `queued`/`running`;根 CA 已过期不可签发)。**禁用按钮必须 `Tooltip` 给出原因**并在可行时提供跳转入口。
- **H5 · 破坏性二次确认**:删除(证书 / 域名)、吊销、移除账户、取消任务一律 `AlertDialog` 二次确认,写清后果(§7.4)。
- **H6 · 风险提示**:导出私钥、监听地址设为对外可达——`danger` Alert + 勾选确认后方可执行(project §7 / roles §3)。已过期根 CA / 移除被引用账户 / DNS-01 等待——`warning` Alert 引导。
- **H7 · a11y**:交互经 Radix primitives(键盘导航 / 焦点管理 / aria);图标按钮必带 `aria-label`;对比度 ≥ WCAG AA(软底 Badge 的 muted-fg 已按此定)。
- **H8 · 技术值用 mono**:指纹 / 序列号 / TXT 记录名值 / webroot 路径 / 监听地址端口 / 证书链 / 执行日志一律 `font-mono`;可复制值配 §7.10 复制范式。
- **H9 · 明暗双套**:每个颜色都有 `.dark` 对应值;原型两套下均须可读、达标;切换经根节点 `.dark` class(§1.2 / §2)。
- **H10 · 一套视觉、形态差异仅显隐**:桌面 / 服务器不产生第二套 token;「仅桌面 / 仅服务器」元素(开机自启 vs 监听端口;托盘角标 vs 浏览器红点;导出原生保存 vs 浏览器下载)是页面级显隐标注,运行形态取自 API(desktop/server)。
- **H11 · 时间显示统一**:数据契约为 RFC3339 UTC(TECH §3.5),**显示**统一为「相对时间 + `Tooltip` 绝对时间」,全端一致;绝对时间用 mono。
- **H12 · 不越 PRD 边界加料**(逐条锚定,防原型漂移):
  - 统计卡片**仅** dashboard 三指标;证书 / 域名 / 任务 / 根 CA **列表页禁放统计卡片**。
  - dashboard **无分布图表**(按签发方式 / CA 的图表未获授权,chart token 仅预留)。
  - **全端禁批量操作**(批量签发 / 吊销 / 删除 / 重试 / 取消均被各 PRD 排除);不放全选框 / 批量操作条。
  - **无全局通知中心 / 通知铃**(MVP 通知收敛为 dashboard 红点)。
  - 设置页**单页分区(Card)非 Tabs**;无账户 / 登录 / 权限、无主题外的语言 / 日志级别等未明示项;运行形态与数据存储路径**只读不可改**。
  - 列 / 字段 / 操作**不超出**对应页面 PRD 的 F 清单。

---

## 11. 决策记录(append-only)

> 只增不改;记「定了什么 / 为什么」。本系统为**全新端**,无既成原型,依 approved 基线(TECH §1.4)与项目定位提出初版并立法。

- **DS1 · 中性冷 slate + 蓝主色**:中性走 hue≈260 冷调、主色蓝 hue≈258。为什么:证书 / HTTPS / 基础设施工具,蓝传达信任与技术感;冷中性衬托状态色。主色蓝(实心按钮)与 info 状态蓝(hue≈240 软底 Badge)**用不同色相 + 不同呈现(实心 vs 软底)**区分,不混淆。
- **DS2 · 状态色语义单表立法 + StatusBadge 单一入口**:5 台状态机全部态 → 5 语义级(§3.2),原型 / 前端只经 `StatusBadge` 渲染。为什么:状态是本产品第一信息;散落手写色值 = 立法失效 + 漂移,与 TECH §4 枚举单一真相精神一致(色也应单一真相)。
- **DS3 · info 内「排队(静态时钟)vs 执行中(旋转 spinner)」分图标**:同蓝色语义,图标区分「等待被处理」与「正在处理」。为什么:queued/pending 与 running/validating 对使用者意味不同(前者等调度,后者已在跑),纯颜色无法传达。
- **DS4 · `awaiting_manual` 归 warning 而非 info**:DNS-01 等待手动配置虽处 flow「进行中」,视觉归**关注 / 等待**(amber)。为什么:它是「等你去加 TXT」的待处理项,与 dashboard 待处理 / 红点级别一致(flows/acme DA2 / dashboard §3),归蓝色进行中会弱化其「需你介入」。
- **DS5 · 分类标签用中性、不占语义色**:签发方式 / 通配符 / 触发方式 / 环境 / 创建方式走中性 outline。为什么:语义色是稀缺信号,只留给生命周期状态与主操作;分类若也上色会淹没状态信号(P2 防彩虹汤)。
- **DS6 · 紧凑密度 + 14px 主力字号 + 边框优先**:工具型信息密集,body 默认 `text-sm`、行高 44px、以 `border` 分区少用重阴影。为什么:证书 / 任务 / 域名列表信息密集(project 工具定位),密度优先于留白;扁平边框比投影更「精确 / 工具感」。
- **DS7 · 破坏性两步 + 高风险追加勾选**:破坏性操作 ghost/outline 触发 → AlertDialog destructive 确认;私钥导出 / 公网暴露再加勾选确认。为什么:project §7 数据安全 + roles §3 公网暴露风险要求「显式化」,分级确认与风险度匹配。
- **DS8 · dashboard 强调色只在计数>0 出现**:三指标健康时中性,`失败数>0`/`即将到期数>0` 才染色。为什么:常态健康不应满屏报警;告警权重(danger>warning,已过期居首)只在真有待办时凸显(flows/dashboard §3.1)。
- **DS9 · 挑战进度用步骤 / 时间线,禁假进度条**:无百分比语义。为什么:HTTP-01 自动 / DNS-01 手动都是「状态推进」而非「进度百分比」;步骤图 + 状态图标如实反映挑战状态机,假进度条误导。
- **DS10 · 无既成端,反向提炼不适用,取基线立初版**:本端全新,依 TECH §1.4 锁定的 shadcn + Tailwind v4 + lucide + sonner 与 project 工具定位提出初版 token;送审后即为全端原型上游。为什么:符合「全新端依 approved 基线提初版并立法」的端级设计系统任务定义。
- **DS11 · 「蓝图 Blueprint」基调:暖纸亮面 + 群青主色(修订 DS1 色相选择)**:亮面底色改暖白(oklch hue≈90 微暖),前景与中性走墨蓝(hue≈265),主色改群青(hue≈268、彩度升至 0.20)。为什么:纯白 + 冷 slate + 标准蓝是「默认 shadcn 脸」,无记忆点;暖纸底让界面像「图纸」,群青即 blueprint 蓝,契合证书 / 基础设施的「可信蓝图」意象,且暖底缓解长时间盯看的冷白光疲劳。DS1 其余论证(蓝系传达信任、冷/墨中性衬托状态色)仍然成立,仅色相与彩度取值被本条修订。
- **DS12 · info 状态色移至天青 hue≈222(修订 DS1 中 240 取值)**:旧值 info(240)与主色(258)仅 18° 色差,软底 Badge 与主色实心按钮在边缘情形(小尺寸、暗色、色弱)仍显含混;新值 222 与群青 268 拉开 46°,且天青比蓝更「过程感」。为什么:主色 = 你要按下的动作,info = 正在发生的过程,二者是界面里最容易同框的两种蓝,必须一眼可分;实心 vs 软底的形态差异保留为第二道区分。
- **DS13 · 锐利化 + 微交互:radius 8→6px,按钮 active 压缩,selection 染色**:圆角基准降至 0.375rem,按钮加 `active:scale-[0.98]` 与 `shadow-xs`(default/destructive/outline),全局 `::selection` 取 20% 主色、滚动条细化。为什么:6px 圆角 + 发丝描边是「精密仪器」语义的形态落点;active 压缩给点击即时物理反馈,成本为零;selection 染色让群青渗透到文本操作细节,统一品牌触感。
- **DS14(2026-07-19)· v2「极光守护 Aurora」整体重塑(修订 DS11 基调、DS13 圆角,落地于 `frontend/src/index.css` v2)**:「蓝图」暖纸 + 群青在双主题下仍偏「黑白灰切换」,缺整体设计感与记忆点;v2 以**深空靛 × 极光青品牌渐变**为基调重写色板与组件形态。
  - **基调**:品牌 = `primary`(深空靛 hue≈267)→ `aurora`(极光青 hue≈197)的 135° 渐变(`.brand-gradient`),用于主按钮、品牌标、侧栏激活指示、一级强调字(`.text-aurora` 渐变文字);浅色「曜石白」/ 暗色「深空墨」共享同一语义 token 结构,暗色下主色提亮保对比。
  - **材质**:卡片与浮层从「扁平边框」升级为**薄玻璃**(`.glass` = 86% 半透明 + `backdrop-blur` +  saturate,配内切边高光);弹窗 / 下拉 / 气泡 / toast 统一玻璃质感;页面底层加 `.ambient` 环境光斑(径向渐变固定层)与 dashboard hero `.grid-paper` 网格纸纹理。
  - **字体**:引入 Space Grotesk(展示 / 标题 / 指标数字,`font-display`)、Outfit(正文)、JetBrains Mono(技术值,承接 H8),经 Google Fonts CDN;系统栈仍作回退。
  - **圆角与阴影**:基准回升至 `0.625rem`(10px;修订 DS13 的 6px),卡片 `rounded-2xl`、浮层 `rounded-xl/2xl`;新增分级阴影 token(`--elevation-card` / `--elevation-pop`)与 `--glow-primary` 品牌辉光,主按钮 / 激活态带微光。
  - **动效**:路由切换 `page-in`、内容入场 `rise-in`、运行形态脉冲点 `pulse-dot`;全部遵循 `prefers-reduced-motion` 降级(H7 延伸)。
  - **不变**:状态色语义(§3 五语义级 + StatusBadge 单一入口,H2)、交互四态(H3)、破坏性两步(H5)、一套视觉形态差异仅显隐(H10)等硬约束全部沿用;v2 只重写色板 / 材质 / 字体 / 动效层,不改状态立法。
  - 为什么:工具型不等于无设计;「证书守护 HTTPS」的产品意象与极光渐变(守护光束)同构,品牌渐变 + 玻璃材质在明暗双主题下都有强识别度,回应「整体设计感 / 科技感 / 高级感」诉求;玻璃与渐变全部落在 token / 工具类层(§1/§2),原型仍禁写死色值(H2 不破)。