# 页面设计提示词 · 证书详情页(certificates / detail)

> 类型: 工作底稿(仅 output 登记,不送审)· 端: app · 模块: certificates · 撰写: designer
> 依据(approved,直接消费):
> - 页面 PRD `docs/prd/pages/app/certificates/detail.md`(F1 信息、F2–F8 生命周期操作、验收 1–13、**§5 脚注:吊销不含已过期**)
> - API 契约 `docs/architecture/api/app/certificates.md`(`CertificateDetail` DTO、各动作适用源态、导出 `parts`/`acknowledgeKeyExport`、`activeTaskId`;**原型不写路径**)
> - 设计系统 `docs/design/systems/app.md`(§3 状态色 / §7.1 Button / §7.4 Dialog·AlertDialog / §7.8 Alert / §7.10 可复制技术值 / §10 硬约束)
> - 证书状态语义与流转引用 systems §3.2 / flows/certificates §2.3;不复述。

---

## 1. 页面目标

单张证书**完整信息** + 其**生命周期操作**落点(续签 / 重试 / 吊销 / 删除 / 导出 / 重新签发)。各操作是否可发起严格由**当前状态**决定(引用 flows/certificates §2.3 权威转移表),页面据此动态可用 / 禁用(§10-H4)。

---

## 2. 布局结构

- **App Shell**:活动项「证书」;面包屑「证书 / {主域名}」。
- **内容区**(单列 `max-w-3xl` 或两栏,§5.2):
  1. **页首**:主域名(`page-title`)+ 当前状态 `StatusBadge` + **操作按钮行**(按状态可用 / 禁用)。
  2. **信息卡**(F1):关联域名 / 当前状态 / 有效期 / 签发方式与来源 / 证书链概要 + 指纹 / 序列号 / 时间戳。
  3. **交互与状态设计态**区(说明性):导出面板 Dialog、吊销/删除 AlertDialog、操作可用性随状态(H4)、失败态原因入口(F8)——静态平铺预览,运行时按状态动态呈现。

---

## 3. 信息卡字段(F1,引用 `CertificateDetail`)

| 字段 | DTO | 呈现 |
| --- | --- | --- |
| 关联域名 | `domains[]`(`hostname`/`isWildcard`) | 逐个列出;通配符加中性 outline「通配符」;每个 `link` → 域名详情(`ArrowUpRight`) |
| 当前状态 | `status` | StatusBadge(§3.2) |
| 有效期 | `notBefore` / `notAfter` / `daysUntilExpiry` | 生效 / 失效绝对时间 mono(§10-H8)+ 相对(§10-H11) |
| 签发方式与来源 | `issuanceMethod` + `acmeAccount{caLabel,environment}` \| `rootCa{name}` | ACME → CA 标签 + 环境(生产中性 / 测试 warning outline,§3.5);自签 → 根 CA 名 |
| 证书链概要 | (链摘要) | mono:叶子 → 中间 CA → 根(§10-H8) |
| 指纹 / 序列号 | `fingerprint` / `serialNumber` | mono + **可复制块**(§7.10:`Copy`→`Check` + Tooltip 全形) |
| 时间戳 | `issuedAt` / `createdAt` / `updatedAt` | 相对 + Tooltip 绝对(mono) |

> **密钥边界**:DTO 无私钥 / PEM 字段;证书链 / 私钥仅经导出取得(不在信息卡展示密钥材料)。

---

## 4. 生命周期操作(F2–F8,§10-H4;适用源态引用 flows §2.3)

按钮语义与 §7.1 变体:

| 操作 | 可发起源态 | 变体 | 图标 | 确认 / 风险 |
| --- | --- | --- | --- | --- |
| 续签 / 重试续签(F2) | 有效 / 即将到期 / 续签失败 / 已过期 | 主行动为 `default`;非紧急态 `secondary` | `RotateCw` | 续签失败态语义「重试续签」 |
| 重试首签(F3) | 签发失败 | `default` | `RotateCw` | — |
| 吊销(F4) | 有效 / 即将到期 / 续签失败(**不含已过期**,§5 脚注) | `ghost` + `text-danger` 触发 | `Ban` | **AlertDialog** 二次确认,确认按钮 `destructive`(§10-H5) |
| 重新签发(F5) | 已吊销 | `default` | `RotateCw` | 为同域名换新私钥 |
| 删除(F6) | 非进行中态 | `ghost` + `text-danger` 触发 | `Trash2` | **AlertDialog** 二次确认;进行中态**禁用** + Tooltip「须先在任务处取消或等任务结束」+ 跳任务(§10-H4/H5) |
| 导出(F7) | 有效/即将到期/续签中/续签失败/已过期/已吊销(仅本地有文件 `isExportable`) | `outline` | `Download` | **Dialog** 选 `parts`;含私钥 → `danger` Alert + 勾选 `acknowledgeKeyExport`(§10-H6) |
| 查看关联任务 / 失败原因(F8) | 进行中态 + 失败态 | `link` | `ArrowUpRight` | 跳最近任务;失败态并展 `lastError` |

- **禁用规则(§10-H4)**:不适用态的操作**隐藏**(如非签发失败不显示「重试首签」、非已吊销不显示「重新签发」);受状态阻断的操作**禁用 + Tooltip 给因**(如已过期的「吊销」禁用、进行中的「删除」禁用)。
- **进行中态**(签发中/续签中/吊销中):`activeTaskId` 存在 → 删除禁用并给「查看任务」跳转;导出按 `isExportable` 定(续签中有文件可导)。

---

## 5. 关键弹窗(§7.4)

- **导出面板(Dialog)**:内容选择 checkbox——叶子证书 / 证书链(默认)/ 私钥。勾选「私钥」→ `danger` Alert(私钥高度敏感)+ 勾选「我已了解风险」后「导出」方可用(禁用态默认,§10-H6/H7)。格式 PEM。形态注:桌面保存本地 / 服务器下载(§10-H10)。
- **吊销确认(AlertDialog)**:写清后果(CA 标记作废、依赖服务中断、不可撤销)、取消 `secondary` + 确认吊销 `destructive`。
- **删除确认(AlertDialog)**:写清后果(移除条目与本地文件、退出状态机、历史任务只读保留)、确认删除 `destructive`;删除成功返回列表(验收 13)。

---

## 6. 交互与四态(§10-H3)

- **normal**(主渲染):一张**有效** ACME 通配符证书完整信息 + 可用操作(续签 secondary / 导出 outline / 吊销 ghost-danger / 删除 ghost-danger)。
- **loading**:字段骨架(`Skeleton`)。
- **empty**:不适用(单资源;不存在 → 404 页,非本页空态)。
- **error**:拉取失败 → `destructive` Alert + 「重试」。
- **disabled**:见 §4 禁用规则,设计态区演示(已过期吊销禁用、进行中删除禁用)。

---

## 7. 视觉 token 引用

状态色 §3.1/§3.2;按钮变体 §7.1;Dialog/AlertDialog §7.4;Alert 变体 §7.8;可复制技术值 §7.10;mono §4.1;间距 §5.2(详情 `max-w-3xl`、卡片 `p-6`);图标 §9.2。颜色走 token(§10-H2)。

---

## 8. 硬约束核对点(本页相关)

- H2 无写死色值;状态经 StatusBadge。
- H3 四态(normal + loading/error;单资源无 empty)。
- **H4 操作按状态禁用**:吊销不含已过期、进行中不可删、导出仅本地有文件态;禁用按钮 Tooltip 给因 + 可行时跳转。
- **H5 破坏性二次确认**:删除 / 吊销 AlertDialog。
- **H6 风险**:导出私钥 danger Alert + 勾选确认。
- H8 指纹/序列号/链/时间 mono;H11 相对 + Tooltip 绝对。
- H10 导出形态差异(桌面保存 / 服务器下载)为页面级标注。
- **H12 不越界**:操作 = F2–F8,不加 PRD 未列动作;无批量;无自动部署。

---

## 9. 原型示例数据(有效证书,主渲染)

- 主体:`valid` · ACME · Let's Encrypt · 生产;域名 `example.com`、`*.example.com`(通配符);生效 2026-06-20、失效 2026-09-18(64 天后);序列号 `03:AC:7F:...`(mono);指纹 SHA-256 `A1:B2:...`(mono);链「叶子 → Let's Encrypt R11 → ISRG Root X1」;`isExportable=true`;`activeTaskId=null`。
- 设计态区补充样例:已过期(吊销禁用)、续签中(删除禁用 + 查看任务)、续签失败(`lastError`「ACME 速率限制,续签被拒 · 2026-07-14」+ 查看任务)。
- 基准日期 2026-07-16。

---

## 10. 边界(本页不含,PRD §2)

批量操作、证书自动部署(导出后自行部署)、任务取消动作本身(在任务模块;本页仅跳转入口)。
