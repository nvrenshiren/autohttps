# autohttps · 逐科设计系统接入

本文件记录 autohttps 如何接入 whzhuke 逐科设计系统。品牌、颜色和组件样式遵循设计系统；页面结构由 autohttps 的产品需求和使用任务决定。

设计系统正式入口为 `D:\Work\dawi\design`，规范见其 `README.md` 与 `DESIGN.md`；业务仓库内的主题文件是供构建使用的快照。

## 主题资源

- `frontend/src/theme/cohesion-shadcn-tokens.css` 是 shadcn 浅色与深色主题的生成快照，源自设计系统的 `theme/create-shadcn-theme.mjs`。
- `frontend/src/theme/cohesion-tokens.css` 提供成功、警告、错误和信息等语义 Token，源自设计系统的 `theme/create-theme.mjs`。
- 运行时主题状态由 `frontend/src/stores/ui.ts` 管理；根节点同步 `data-theme` 与 `data-shadcn-theme`。浅色、深色和系统偏好均使用上述 Token。
- 页面与业务组件使用语义 Token，不复制基础色值或另建品牌色板。产品状态色保持独立，不用品牌色替代成功、警告或失败语义。
- 前端构建使用仓库内的主题快照，不依赖开发机上的设计系统绝对路径。设计系统更新后，应从正式配置重新生成完整快照，并核对消费侧主题。

## 组件与视觉约定

- 组件体系使用项目现有的 shadcn/ui、Radix 和 Tailwind CSS v4。优先复用 `frontend/src/components/ui/` 中的控件与语义 Token。
- 使用系统字体栈；域名、指纹、序列号、TXT 记录、路径和时间等技术值按需使用等宽字体。
- 主操作使用品牌主色，次要操作使用中性样式，危险操作使用危险语义。状态标签使用统一状态组件，分类标签不冒充生命周期状态。
- 长内容、表单、菜单、对话框、Tooltip 和 Toast 使用清晰的不透明表面。装饰效果不能干扰内容对比度或操作辨识。
- 交互保留键盘操作、可见焦点、正确标签与屏幕阅读器语义；减少动态效果偏好下不播放非必要动画。

## 页面结构与导航

- 页面、导航和信息层级根据产品需求中的功能、用户任务与平台范围确定，不以现有页面结构或路由作为模板。
- 列表可按信息密度采用清单、卡片行或表格；详情页突出当前对象及可执行的下一步，不要求各模块套用同一模板。
- 不新增 PRD 未定义的字段、统计、批量操作或状态。桌面、服务器和移动 Web 共用品牌与组件规则，仅按运行形态显示相应功能。

## 产品图标

`assets/icon.svg` 是 autohttps 产品图标；前端入口为 `frontend/public/icon.svg`。桌面尺寸 PNG 由 `tools/render_icons.py` 生成；`frontend/public/favicon-32.png` 单独维护。
