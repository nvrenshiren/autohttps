---
name: hickory-resolver-dns-precheck
description: dns-precheck 用 hickory-resolver 本地查 TXT;feature 配置避 aws-lc-rs、0.25 vs 0.26 版本坑、API、验证时的 DNS 坑
metadata:
  type: project
---

`GET /acme/challenges/{id}/dns-precheck`(acme api §2.3,B4 可选)实现:core `services::acme::dns_precheck`
用系统 DNS 解析器查挑战 `dns_txt_name` 的 TXT,判 `dns_txt_value` 是否已生效 → DTO `{propagated, observedValues}`。

- **当前版本**:`Cargo.toml` 约束为 hickory-resolver `0.25`,代码按 0.25 API 编写。当前 workspace MSRV 为
  Rust 1.88,升级不再受当时的 MSRV 限制；变更前仍需复核新版本 API、特性与解析行为。
- **feature 配置(保 ring-only Windows 构建,L7 底线)**:`default-features=false, features=["system-config","tokio"]`。
  默认就只有这俩;所有 crypto backend(`tls-ring`/`tls-aws-lc-rs`/`dnssec-*`/`https-*`/`quic-*`)**全非默认**。
  纯 UDP/TCP 查询**不启用任何 TLS/DNSSEC/DoH** → **不引入 aws-lc-rs、不引入 rustls-crypto**(`cargo tree -i aws-lc-rs`
  报 "did not match any packages" = 树中无)。既有 rustls(instant-acme hyper-rustls + sqlx runtime-tokio-rustls)仍 ring。
  当前 feature 集仅用于系统 DNS 配置和 Tokio 查询；新增或调整 feature 时检查依赖树,避免不必要地引入 `aws-lc-rs`。
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
- **验证注意**:测试 `propagated:true` 时优先使用受控域名和已知 TXT 值；公共 DNS 记录可能轮换、截断或变化。
  预检只按 challenge ID 读取挑战并查询 TXT,不需要启动 Pebble/ACME。构造测试数据时需提供 challenges 表的必填字段。

**检查状态**:`cargo clippy` 是 CI 阻断检查并已通过；若出现 lint,按当前错误定位修复,不要依赖旧记忆中的
`never_loop` 或文档续行告警。
