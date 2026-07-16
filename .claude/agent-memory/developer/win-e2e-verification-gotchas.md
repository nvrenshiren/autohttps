---
name: win-e2e-verification-gotchas
description: Windows/MSYS 自验证踩坑:Python 读 UTF-8 用 -X utf8;系统 openssl.cnf v3_ca 坏,建 x509 用 minimal -config;server.exe 常驻要先 taskkill 再 build
metadata:
  type: project
---

curl 自验证(起 server + curl 实测)在本机(Win11 + MSYS/Git Bash)反复踩的三坑,避免重复浪费轮次:

- **Python 解析含中文的 curl JSON 响应必须显式 UTF-8**:本机 Python 默认 GBK,`json.load(open('x.json'))`
  直接 `UnicodeDecodeError`。用 `python -X utf8` 且 `open(..., encoding='utf-8')`。API 的 message/resultSummary
  是中文,几乎必踩。
- **系统 openssl.cnf 的 `[v3_ca]` 段在本机是坏的**(`authorityKeyIdentifier=keyid` 报 unknown option)。
  要造带扩展的 x509(如导入用的 CA 证书)时,别用 `-addext`/默认 config,写个 minimal cnf 用
  `OPENSSL_CONF=mini.cnf openssl req -x509 ...`(段含 `basicConstraints=critical,CA:TRUE` + `keyUsage`)。
  私钥用 `openssl genpkey -algorithm EC`(出 PKCS#8 `PRIVATE KEY`,rcgen `KeyPair::from_pem` 认;SEC1 未必认)。
- **重建 server 前先杀常驻进程**:上一轮后台 `server.exe` 不退,`cargo build -p server` 会
  `failed to remove ...server.exe (os error 5 拒绝访问)`。先 `tasklist //FI "IMAGENAME eq server.exe"`
  →`taskkill //F //PID <pid>`。后台起法:`AUTOHTTPS_DATA_DIR=... AUTOHTTPS_ADDR=127.0.0.1:<port> ./target/debug/server.exe >log 2>&1 &`。

执行器(self_signed)约 500ms 轮询即跑完,`POST /certificates` 后 poll `GET /certificates/{id}` 一两次即 `valid`。
