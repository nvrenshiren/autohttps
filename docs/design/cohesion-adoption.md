# autohttps · COHESION 设计系统接入

前端以现有 shadcn/ui、Radix 和 Tailwind CSS v4 作为唯一组件体系，品牌视觉跟随 **COHESION 逐科设计系统 v2.0.0**。页面继续按 autohttps 的证书管理任务组织；本接入文档记录项目实际采用的主题、Shell 和组件表达。

## Token 与品牌资源

- shadcn 浅深主题快照来自设计系统 `theme/create-shadcn-theme.mjs` 生成的 `preview/shadcn/tokens.css`，接入文件为 `frontend/src/theme/cohesion-shadcn-tokens.css`。
- 状态语义读取 `theme/create-theme.mjs` 的官方算法结果；对应 `dist/tokens/cohesion-tokens.css` 随项目打包为 `frontend/src/theme/cohesion-tokens.css`，成功、警告、错误、信息各自保留语义色族。
- COHESION 浅色与深色品牌标志分别来自 `原色.png`、`白色.png`，保存在 `frontend/public/`。应用主操作使用派生 `primary` 与独立 hover/active 角色；状态色不承担品牌色用途。
- `:root` 同步 `data-theme` 与 `data-shadcn-theme`，密度设为 `comfortable`；`frontend/src/stores/ui.ts` 是运行时主题状态入口。两个 Token 文件和 PNG 均随前端构建，运行时不引用设计系统工作区绝对路径。
- 主题来源变化后，在 `D:\Work\dawi\design` 更新相应生成物，再将完整输出复制到上述消费路径并检查版本差异。禁止在页面另手写一套主题基础色。

## 页面与组件表达

- 画布使用很低对比的品牌光晕；长内容、对话框、输入和数据列表使用清楚的不透明表面。
- 页面标题与正文使用系统字体栈，不在线加载展示字体。页面主标题、辅助说明、状态标签和等宽技术值按层级区分。
- 主按钮使用实色品牌绿，次要按钮使用中性面；实体危险操作单独使用危险按钮配色。标签继续区分状态与普通分类。
- 证书、任务、账户和挑战的状态含义由现有状态映射与 API 契约决定；主题改造不改变其流转、筛选或可执行操作。
- 桌面在 `xl` 展开为约 240px 侧栏，较窄窗口使用 64px 图标栏；手机底栏提供总览、域名、证书、任务 4 个常用入口，“更多”菜单承载 ACME、根 CA 和设置。800×600 Tauri 基线下保留全部桌面路由入口。
- 手机输入与主操作提供至少 44px 的点击高度，窄屏输入保留 16px 字号；表格保留局部横向滚动。
- 路由切换仅对内容区域轻入场；系统减少动态效果时页面、状态、加载提示和浮层均直接显示最终结果。

## 项目品牌图标

`assets/icon.svg` 保留 autohttps 锁与验证勾图形，底色和勾色使用 COHESION 浅色 `primary` 与 `primary-active`。`tools/render_icons.py` 是桌面各尺寸图标的生成源，前端 favicon 取其 32×32 输出。

## UI 设计与复核

- 逐科 UI 设计、实现与调整同时遵循 COHESION 设计系统，并使用 `ui-ux-pro-max` 做针对性的布局、响应式、交互与无障碍复核。
- COHESION 是品牌、token 和组件表达的规范来源；`ui-ux-pro-max` 用于发现问题与补充实践建议。搜索结果需结合本项目业务和现有技术栈判断，不直接覆盖 COHESION 的正式规范。
