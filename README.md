<div align="center">

# autohttps

**跨平台 HTTPS 证书全生命周期管理工具 · 桌面 + 服务器双形态**

![Rust](https://img.shields.io/badge/Rust-1.9x-CE412B?logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-19-149ECA?logo=react&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Tailwind CSS](https://img.shields.io/badge/Tailwind-v4-38BDF8?logo=tailwindcss&logoColor=white)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-555)

**中文** · [English](README.en.md)

让「维护一批域名 + 一组证书」这件事变得省心 —— 启动即检测、到期前自动续、失败有清晰提示。

</div>

---

## ✨ 特性

- **两种签发方式**
  - **ACME 公共 CA**(Let's Encrypt 等):账户注册、**HTTP-01(webroot)** 自动验证、**DNS-01(手动)** 展示 TXT 记录引导添加。
  - **自签根 CA**:创建 / 导入根 CA,为内网 / 本地开发统一签发可信证书,导出根 CA 供客户端信任。
- **证书全生命周期**:签发 → 有效 → 到期 → 续签 / 重试 / 吊销 / 删除 / 导出(叶子 / 证书链 / 私钥,私钥导出带风险确认)。
- **省心自动化**:启动即全量扫描;到期前按策略**自动续签**;进程崩溃后启动自恢复,任务不卡死。
- **实时可见**:总览首屏三指标(总数 / 即将到期 / 失败)+ 待处理清单 + 红点;SSE 实时推送,状态一变界面就刷新。
- **两种运行形态,一套前端**
  - **桌面形态**(Tauri):800×600 窗口 + 系统托盘常驻、关窗不退出、开机自启、托盘红点角标。
  - **服务器形态**:守护进程 + 浏览器 Web UI,7×24 常驻。
- **数据安全**:私钥 / ACME 账户密钥 / 根 CA 私钥经 [age](https://github.com/FiloSottile/age) 加密静态存储,数据库内只存引用、绝不明文,日志脱敏。

---

## 🚀 快速开始

前置:Rust(1.9x)、Node.js(20+)。

```bash
# 1) 构建前端(产物内嵌进可执行文件)
cd frontend && npm install && npm run build && cd ..

# 服务器形态:浏览器访问 Web UI
cargo run -p server          # → http://127.0.0.1:8443

# 桌面形态:800×600 窗口 + 系统托盘
cargo run -p desktop
```

环境变量:`AUTOHTTPS_ADDR`(监听地址,默认 `127.0.0.1:8443`)· `AUTOHTTPS_DATA_DIR`(数据目录,默认 `./data`)· `AUTOHTTPS_ACME_CA_CERT`(测试用:信任自定义 ACME CA 根证书)。

### 用真实 CA 签发

签发公共证书时把 ACME directory 指向真实 CA 即可,例如 Let's Encrypt:`https://acme-v02.api.letsencrypt.org/directory`。

---

## 🏛 架构

**一套 Rust 核心 + 一套 React 前端**,两形态共享,差异仅在「外壳」。前端**始终且只**经 HTTP + SSE 通信;桌面形态在进程内内嵌同一 axum 服务(仅回环),两形态挂载**同一** Router —— 契约只定义一次(方案 A)。

```
crates/
  core/      领域核心:5 台状态机枚举(单一真相)· SQLite 持久化(SeaORM)· ACME 客户端
             · 自签 CA(rcgen)· age 敏感数据存储 · 任务队列 + 执行器 · 到期扫描器
  api/       传输契约:axum Router(REST + 全局 SSE /events)· DTO · 内嵌 SPA
  server/    服务器形态守护进程(bin)
  desktop/   桌面形态 Tauri v2 壳(bin,内嵌 api/core)
frontend/    React 19 + Vite + TypeScript,两形态共用;经 react-query 打 /api
docs/        prd(需求契约)· architecture(DB / API 契约)· design(设计系统)
```

**任务队列即 SQLite 表**(天然持久化 + 崩溃可恢复);**枚举单一真相**定义在 `core`、导出到前端;敏感数据只存 `*_ref`,密文落数据目录。

---

## 🧪 用 Pebble 本地测 ACME

无需真实域名 / 公网,用 Let's Encrypt 官方测试服务器 [Pebble](https://github.com/letsencrypt/pebble) 本地验证 ACME 客户端全流程:

```bash
# 构建并以 always-valid 模式运行 Pebble(挑战自动判过)
git clone --depth 1 https://github.com/letsencrypt/pebble && cd pebble
go build -o pebble.exe ./cmd/pebble
PEBBLE_VA_ALWAYS_VALID=1 ./pebble.exe -config test/config/pebble-config.json   # → https://localhost:14000/dir

# autohttps 侧信任 Pebble 的 CA 根证书
AUTOHTTPS_ACME_CA_CERT=<pebble>/test/certs/pebble.minica.pem cargo run -p server
```

在 UI 里注册账户(directory 填 `https://localhost:14000/dir`)→ 签 ACME 证书 → HTTP-01 自动通过 / DNS-01 按向导确认 → 证书转「有效」。

---

## 🧱 技术栈

| 层 | 选型 |
| --- | --- |
| 后端 | Rust · axum · SeaORM + SQLite(WAL)· tokio · [instant-acme](https://github.com/instant-labs/instant-acme)(ACME)· [rcgen](https://github.com/rustls/rcgen)(X.509 / 自签)· [age](https://github.com/str4d/rage)(加密)· rust-embed(内嵌 SPA) |
| 桌面 | Tauri v2(tray / single-instance / autostart 插件) |
| 前端 | React 19 · Vite · TypeScript · Tailwind CSS v4 · shadcn/ui + Radix · @tanstack/react-query(服务端态)· zustand(客户端态)· react-hook-form + zod · lucide-react · sonner |

---

## 📦 项目状态

MVP 核心功能完整可用(自签 + ACME 两条主线,桌面 + 服务器两形态),证书签发 / 续签 / 吊销 / 导出、到期扫描 / 自动续签、实时刷新均端到端验证通过。

**非目标(MVP 明确不做)**:证书自动部署到 nginx / apache(止于导出,由用户自行部署)· 多渠道通知(仅红点)· Web UI 鉴权(默认仅本机 / 可信内网)· DNS-01 厂商 API 自动验证(仅手动)。

---

## 📄 License

[MIT](LICENSE)
