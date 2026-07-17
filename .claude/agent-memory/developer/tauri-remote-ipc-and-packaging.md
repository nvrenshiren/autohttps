---
name: tauri-remote-ipc-and-packaging
description: 桌面壳扩展 JS IPC 的远程 URL capability 坑(dialog/fs 导出)+ tauri build 打包实操(CLI/bundler 自下载/产物路径)
metadata:
  type: feedback
---

给桌面壳新增任何 **Tauri JS IPC** 调用(如原生导出 dialog `save()` + fs `writeFile`)时的非显然约束;以及 `tauri build` 打安装包的实操要点。见 [[tauri-desktop-shell]]。

**Why:** 桌面壳的 WebView 加载的是**回环 http 远程 URL**(`http://127.0.0.1:38443`,方案A),**不是** `tauri://` 本地内容。这让 IPC 授权模型与常规 Tauri 应用不同,错了运行期直接 "command not allowed" 或写盘 PathForbidden,且**默认 capability 看起来该覆盖却不覆盖**,极易误判。

**How to apply:**
- **远程页面调命令必须建带 `remote.urls` 的独立 capability**:现有 `capabilities/default.json` 的 `core:default` 只对本地(tauri://)内容生效。之前 tray/自启没配 remote 也能用,是因为它们**纯 Rust 侧驱动、前端零 IPC**。一旦前端要 invoke,就得像 `capabilities/export.json` 那样:`"windows":["main"]` + `"remote":{"urls":["http://127.0.0.1:38443/*"]}` + 具体权限。URLPattern `*` 跨段匹配子路径(BrowserRouter 深链如 `/certificates/:id` 都覆盖)。改回环端口要同步改这里的 url。
- **fs `writeFile` 到用户经 `save()` 亲选的任意路径,必须给显式 scope**:`fs:allow-write-file`/`fs:allow-write-text-file` **空 scope = 拒绝一切**(源码 `resolve_path` 里 `is_allowed` 必须命中 allow 才放行,否则 `PathForbidden`)。用 `{"identifier":"fs:allow-write-file","allow":[{"path":"**"}]}`。`**` 在 tauri fs scope(`require_literal_separator:true`)下**匹配任意盘符绝对路径**(C:/D: 均可,已实测);新建/不存在的文件也放行(`try_resolve_symlink_and_canonicalize` 对 `!exists` 原样返回)。这是"授写整盘"的安全折中,内容为本机自有 SPA 故可接受;只授写不授读/删。
- **`isTauri()` 判据是 `window.isTauri`**(非 `__TAURI_INTERNALS__`),由核心 INIT_SCRIPT `Object.defineProperty(window,'isTauri',{value:true})` 设置;远程 URL 经 `onPageStarted` 注入(webview/mod.rs 有 "For remote URLs we use onPageStarted" 注释),所以回环页面 `isTauri()`==true、`invoke` 可用。前端分流:`opts.desktop(来自 /app-info runMode) && isTauri()` 才走原生;插件用**动态 import**(vite 代码分割成 `dist-js-*.js` 小 chunk,server 形态不加载 Tauri JS)。导出函数返回 bool 区分"用户取消 save 对话框"(不弹成功 toast)。

**打安装包(tauri build):**
- CLI 用 `npm i -g @tauri-apps/cli@2`(预编译二进制,秒装;`cargo tauri` 默认没有,`cargo install tauri-cli` 要从源码编译很慢)。
- **非标准布局(`crates/desktop` 而非 `src-tauri`)照样能打**:`cd crates/desktop && tauri build`,CLI 就地找 `./tauri.conf.json`,`frontendDist=../../frontend/dist` 相对它解析。
- **没配 `beforeBuildCommand`**:release 前**必须先 `npm run build`**(rust-embed 在 release 是真内嵌,编译期读 `frontend/dist`)。已验证 release 二进制内嵌的是新 dist。
- **bundler 免手动装**:`bundle.targets:"all"` 下 Tauri **自动下载** NSIS 3.11 + WiX 3.14(GitHub releases)到缓存,无需预装。产物:`target/release/bundle/nsis/autohttps_<ver>_x64-setup.exe`(~6.7M)+ `target/release/bundle/msi/autohttps_<ver>_x64_en-US.msi`(~9.9M)。`target/` 被 gitignore,产物不入库。
- release 全量编译较慢(数分钟),**放后台 + 轮询日志**(cargo.exe 存活即在编;`Finished N bundles at:` 即成功)。
