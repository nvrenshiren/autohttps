# 页面设计提示词 · 证书列表页(certificates / list)

> 类型: 工作底稿(仅 output 登记,不送审)· 端: app · 模块: certificates · 撰写: designer
> 依据(approved,直接消费):
> - 页面 PRD `docs/prd/pages/app/certificates/list.md`(F1–F6、页面流转、验收 1–11)
> - API 契约 `docs/architecture/api/app/certificates.md`(`CertificateSummary` DTO、`GET /certificates` 过滤字段;**原型不写路径**)
> - 设计系统 `docs/design/systems/app.md`(§3 状态色 / §3.5 分类标签 / §7.3 Table / §10 硬约束)
> - 证书状态语义引用 systems §3.2;不复述。

---

## 1. 页面目标

证书管理入口页:全部**未删除**证书一屏总览,支持按状态 / 签发方式 / 域名关键字筛选,提供进入详情、发起首签入口。回答「我现在有哪些证书、各自什么状态」。

---

## 2. 布局结构

- **App Shell**:活动项「证书」;面包屑「证书」。
- **内容区**(`p-6`):
  1. **页首行**:页标题「证书」 + 右侧主操作「发起签发」(`default` `Plus`,F6)。
  2. **筛选工具栏**(§7.3,置表格上方):域名关键字 `Input`(前置 `Search`,F4)· 状态 `Select`(F2)· 签发方式 `Select`(F3);组合生效(验收 7)。
  3. **证书表格**(满幅,§5.2):`DataTable`,底部 `Pagination`。

---

## 3. 表格列(= PRD F1 字段,禁自增列 · §10-H12)

字段引用 `CertificateSummary`;**无统计卡片、无选择框 / 批量列**(§10-H12 / 验收 10、11)。

| 列 | 字段 | 呈现 |
| --- | --- | --- |
| 状态 | `status` | **StatusBadge**(§3.2:变体 + 中文名 + lucide 图标;`issuing`/`renewing` 用 `Loader2` 旋转,`pending_issue` 用静态 `Clock`) |
| 关联域名 | `domains[]`(`DomainRef.hostname` / `isWildcard`) | 多域名(SAN)全列出;通配符加中性 outline「通配符」小 Badge(§3.5);长名 `truncate` + Tooltip 全量 |
| 有效期 | `notAfter` + `daysUntilExpiry` | 相对时间为主(「64 天后 / 已过期 7 天 / —」),Tooltip 绝对(生效 `notBefore` → 失效 `notAfter`,mono §10-H8/H11);未签发态 `notAfter=null` → 「—」 |
| 签发方式 | `issuanceMethod` | 中性 outline Badge:ACME(`KeyRound`)/ 自签(`Landmark`)(§3.5,**不占语义色**) |
| (行尾) | — | `ChevronRight` 导航指示(行主体点击进详情,F5);**无行内生命周期操作**(那些归详情页,DEC3) |

> **不加**:环境(生产/测试)列、来源列、操作菜单列——PRD F1 未列;环境属详情页字段。行操作仅「进详情」(行点击),无 `MoreHorizontal` 菜单(列表页无单证书动作)。

---

## 4. 交互与四态(§7.3 / §10-H3)

- **normal**(主渲染):~10 行覆盖多状态。行 hover 高亮,点击进详情(F5)。
- **loading**:骨架行(`Skeleton`)。
- **empty**:两语气——① 尚无证书(`ShieldCheck` muted +「还没有证书」+ 主 CTA「发起签发」);② 筛选无命中(+「清除筛选」次 CTA)。原型主渲染 normal;empty/loading 可选演示。
- **error**:行区 `destructive` Alert + 「重试」。
- 筛选为查看辅助,不改状态(验收 4–7)。

---

## 5. 视觉 token 引用

状态色 §3.1/§3.2;分类标签中性 §3.5;表格密度 §5.2(行 `h-11`、`px-3 py-2.5`,表头 `h-10` `text-[13px]` muted、sticky);字号 §4.2;图标 §9。颜色走 token(§10-H2)。

---

## 6. 硬约束核对点(本页相关)

- H2 无写死色值;状态经 StatusBadge。
- H3 四态齐(normal + empty 两语气 + loading + error 说明/演示)。
- H8/H11 有效期 mono + 相对/Tooltip 绝对。
- **H12 不越界**:列 = F1(状态/域名/有效期/签发方式);**无统计卡片、无批量 / 全选、无行内单证书操作、不加环境或来源列**。

---

## 7. 原型示例数据(~10 行,覆盖多态)

| # | 状态 | 签发方式 | 域名 | 有效期 |
| --- | --- | --- | --- | --- |
| 1 | valid 有效 | ACME(Let's Encrypt·生产) | example.com, *.example.com(通配符) | 失效 2026-09-18(64 天后) |
| 2 | expiring_soon 即将到期 | ACME | api.example.com | 失效 2026-07-25(9 天后) |
| 3 | expiring_soon 即将到期 | ACME | shop.example.com | 失效 2026-07-28(12 天后) |
| 4 | renewal_failed 续签失败 | ACME | mail.example.com | 失效 2026-07-20(4 天后) |
| 5 | issue_failed 签发失败 | ACME(ZeroSSL) | new.internal.corp | — |
| 6 | expired 已过期 | ACME | legacy.example.com | 失效 2026-07-09(已过期 7 天) |
| 7 | revoked 已吊销 | ACME | compromised.example.com | 失效 2026-09-01 |
| 8 | issuing 签发中 | ACME(测试) | *.staging.example.com(通配符) | — |
| 9 | renewing 续签中 | ACME | cdn.example.com | 失效 2026-07-30 |
| 10 | valid 有效 | 自签(Homelab Root CA) | nas.lan, router.lan | 失效 2027-07-01 |

Pagination:total 24、当前第 1 页(pageSize 10)。基准日期 2026-07-16。

---

## 8. 边界(本页不含,PRD §2)

批量签发 / 吊销 / 删除、证书数量统计卡片(归 dashboard)、单证书的续签 / 吊销 / 删除 / 导出(归详情页)。
