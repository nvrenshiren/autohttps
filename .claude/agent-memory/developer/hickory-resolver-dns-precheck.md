---
name: hickory-resolver-dns-precheck
description: dns-precheck 用 hickory-resolver 本地查 TXT;feature 配置避 aws-lc-rs、0.25 vs 0.26 版本坑、API、验证时的 DNS 坑
metadata:
  type: project
---

`GET /acme/challenges/{id}/dns-precheck`(acme api §2.3,B4 可选)实现:core `services::acme::dns_precheck`
用系统 DNS 解析器查挑战 `dns_txt_name` 的 TXT,判 `dns_txt_value` 是否已生效 → DTO `{propagated, observedValues}`。

- **hickory-resolver 版本坑**:0.26 需 rustc 1.88,workspace rust-version=1.82 → `cargo add`/build **自动降到 0.25.2**。
  按 0.25 API 写(非 0.26)。
- **feature 配置(保 ring-only Windows 构建,L7 底线)**:`default-features=false, features=["system-config","tokio"]`。
  默认就只有这俩;所有 crypto backend(`tls-ring`/`tls-aws-lc-rs`/`dnssec-*`/`https-*`/`quic-*`)**全非默认**。
  纯 UDP/TCP 查询**不启用任何 TLS/DNSSEC/DoH** → **不引入 aws-lc-rs、不引入 rustls-crypto**(`cargo tree -i aws-lc-rs`
  报 "did not match any packages" = 树中无)。既有 rustls(instant-acme hyper-rustls + sqlx runtime-tokio-rustls)仍 ring。
  加 hickory 后 Cargo.lock 只多 `hickory-proto`+`hickory-resolver`(及 moka/ipconfig 等纯 DNS 依赖),aws-lc 计数保持 0。
- **0.25 API**(源码在 `$CARGO_HOME/registry/src/.../hickory-resolver-0.25.2`;`CARGO_HOME=D:\Scoop\user\persist\rustup\.cargo`):
  `hickory_resolver::TokioResolver::builder_tokio()?`(读系统 DNS 配置,Win 读注册表;需 `system-config`+`tokio` feature)
  → `.build()` → `resolver.txt_lookup(name).await`(`name: impl IntoName`,`&str`/`String` 均可)→ `Result<TxtLookup, ResolveError>`。
  `lookup.iter()` 产出 `&proto::rr::rdata::TXT`;`txt.txt_data() -> &[Box<[u8]>]`(单记录的多 char-string,拼接为完整值)。
  NXDOMAIN/超时/无记录 → `Err(ResolveError)`(`never_loop` 无关)。
- **预检语义(flows §4.3)**:**任何**查询失败(NXDOMAIN/超时/无记录/解析器初始化失败)**吞为空 Vec**,
  如实返回 `propagated:false`+`observedValues:[]`,**绝不 500/501**(失败仅 warn/debug 日志)。补末尾 `.` 成 FQDN 避 search 域误查。
- **契约边界**:非 DNS-01 挑战 → `422 not_dns01_challenge`(既有码,非自造)。**契约(acme.md §2.3)只定义这一个拒绝,
  对挑战状态沉默**——故实现**未**按 challenge 状态硬门禁(任意 dns_01 状态都做只读预检);别自造/挪用 `challenge_not_awaiting_manual`
  (那是 confirm 专属码,挪用=偏离 approved 契约)。前端实际只在 awaiting_manual 调用,无害。
- **验证 DNS 坑(自测 propagated:true 用)**:`google.com` 的 TXT 集很大 → 每次 UDP 查询返回**轮换/截断的子集**,
  同一记录未必每次都在 → 不适合做精确匹配的 propagated:true 测试(实测 6 条里没 SPF 那条 → false)。
  改用 **`example.com`**(仅 2 条小而稳:`v=spf1 -all` + `_k2n1y4vw3qtb4skdx9e7dxt97qrmmq9`)→ 期望值填 `v=spf1 -all` 稳定 propagated:true。
  验证不必走 Pebble/ACME:dns_precheck 只 `find_by_id(challenges)`(不 join task/cert),Python sqlite3(默认 foreign_keys=OFF)
  **播一条裸 challenge 行**即可(填 NOT NULL:id/task_id/domain_id/validation_method/status/created_at/updated_at)。

**踩坑**:改中文 doc 注释时,若某行以 `- `/`+ ` 开头(markdown 列表标记),其**换行续接行不缩进** → clippy
`doc_lazy_continuation` 告警(certificates.rs auto_renew 的 `/// + 入队…` 就是)。修法:续接内容**并到同一行**(该仓多为长行),
或续行加缩进。注:本仓 `cargo clippy` 在 master 上**本就红**(executor.rs `tick()` 的 `for` 循环触发 deny 级 `never_loop`
correctness lint + 多处既有 doc_lazy 告警)——**任务验收线是 `cargo check --workspace`+`npm run build`,非 clippy**。
