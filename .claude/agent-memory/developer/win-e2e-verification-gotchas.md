---
name: win-e2e-verification-gotchas
description: Windows/MSYS 自验证踩坑:Python 读 UTF-8 用 -X utf8;curl 发中文 JSON body 被 shell 搞坏(用 ASCII 或 --data-binary @file);openssl.cnf v3_ca 坏用 minimal -config;server.exe 常驻先 taskkill 再 build
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
- **curl 发含中文的 JSON 请求体也会被 shell 搞坏**:Git Bash 里 `curl -d '{"name":"内网根 CA",...}'`
  会让后端报 `validation_failed: invalid unicode code point at line 1 column N`(N 正好落在中文字符处)——
  是 shell 传参的编码问题,非后端 bug(前端 `JSON.stringify` 走的是正常 UTF-8)。自验证时用 ASCII 名,
  或 `printf '{...}' > body.json && curl --data-binary @body.json`。纯 ASCII 字段(如 hostname)不受影响。
- **重建 server 前先杀常驻进程**:上一轮后台 `server.exe` 不退,`cargo build -p server` 会
  `failed to remove ...server.exe (os error 5 拒绝访问)`。先 `tasklist //FI "IMAGENAME eq server.exe"`
  →`taskkill //F //PID <pid>`(或 `taskkill //F //IM server.exe`)。后台起法:
  `AUTOHTTPS_DATA_DIR=... AUTOHTTPS_ADDR=127.0.0.1:<port> ./target/debug/server.exe >log 2>&1 &`。
- **原生 Windows Python 不认 MSYS `/tmp` 路径**:Git Bash 的 `ls`/`find`/`curl -o /tmp/x` 用 MSYS 挂载
  (`/tmp`=`C:/Users/<用户>/AppData/Local/Temp`),但 `python -c "glob.glob('/tmp/...')"` 会当成 `C:\tmp\...`
  → 找不到文件、静默返回 0。用 Python 读产物文件时改用**真实 Windows 路径**(server 日志里的
  `db=C:/Users/.../Temp/<datadir>\autohttps.db` 就是锚点),或先 `ls` 定位再喂绝对 Windows 路径。
  curl `-o` 输出到当前工作目录(cd /tmp 后)再用相对文件名 open 是安全的。

执行器(self_signed)约 500ms 轮询即跑完,`POST /certificates` 后 poll `GET /certificates/{id}` 一两次即 `valid`。

- **验证"取消 queued 任务 / 重试 failed 任务"要确定性种子,别跟执行器竞速**:self_signed 的 issue/renew/revoke
  任务 <500ms 就跑完,想 curl 抢在 `queued` 态取消几乎必输(flaky)。可靠做法——**停服 → Python sqlite3
  直接播种 → 重启**:
  - **常驻 queued 任务**:插一个 `issuance_method='acme'` 的证书 + 其 issue 任务(`status='queued'`)。执行器
    `tick` 对非 self_signed 证书 `continue` 跳过,故 acme 任务**永远 queued**——正好拿来测 cancel。取消后证书
    `pending_issue→issue_failed`(T21)。
  - **failed 任务**:插 self_signed 证书(`status='issue_failed'`)+ 其任务(`status='failed'`),重启后
    `POST /tasks/{id}/retry` 派生新 issue 任务 → 执行器 self_signed 真跑 → 证书 `valid`。
  - **T23/T24 回退**:插 acme 证书(`status='renewing'`/`'revoking'`,`not_after` 设远期如 +300d 避开续签窗)
    + queued renew/revoke 任务;renew 任务带 `parent_task_id`(指向一条 failed renew)则回退判 `renewal_failed`
    (parent 链推断),否则按 `not_after` 有效期回 `valid`/`expiring_soon`/`expired`。
  - WAL 注意:server 用 WAL,**必须先杀 server.exe 再让 Python 开库**(Python `sqlite3.connect` 会自动 WAL
    恢复看到已提交数据;写完 commit,重启 server 即读到)。播种 ID 随便 uuid4 即可(库不校验 UUIDv7 格式)。

验证**扫描器 / SSE**(见 [[build-layout-notes]] 的三层落位):

- **触发扫描别等 60s 周期,重启 server 即可**:boot 序列 `boot::run` 里就跑一次 `scan::scan_once`(启动即全量
  扫描 + 自动续签),所以"改 settings → 杀 server → 重启"能立刻看到扫描结果(比等 `SCAN_INTERVAL`=60s 的周期任务
  确定性得多)。run2 的 `server.log` 会打 `ScanReport { certs_expiring_soon, certs_expired, root_cas_expired, auto_renews_started }`。
- **测自动续签的经典造法**:把 `renewalAdvanceDays` PATCH 成远大于叶子有效期(叶子固定签 365d,设 `100000`)→
  一张 `valid` 自签证书被判 `expiring_soon`(T6)→ `autoRenewEnabled`(默认开)→ auto 续签(T9,`trigger=auto`)→
  执行器重签 → 回 `valid` 且 **serial 变**。⚠**副作用**:advance-days 一直 > 有效期时,**每轮扫描都会再续签**
  (expiring_soon→renew→valid→又 expiring_soon…死循环续),测完**务必把 advance-days 改回 30** 停循环。
- **SSE 抓流**:`curl -N --max-time 6 -s $B/api/events > sse.out &`(`--max-time` 自动断开,免手动 kill),
  sleep 1 确保连上,再触发状态变更(吊销/签发),`wait` 后 `grep -E "^event:|^data:" sse.out`。一次吊销会推
  `certificate_status_changed`(revoking→revoked)+`task_status_changed`(queued→running→succeeded)+多条
  `task_log_appended`+`dashboard_changed` —— payload 是 **camelCase**(`certificateId`/`taskId`/`rootCaId`/
  `pendingCount`/`seq`),仅标识+判别字段,不含 `*_ref`/密钥。boot 期扫描发的事件**无 SSE 订阅者会被丢**(best-effort,正常)。
