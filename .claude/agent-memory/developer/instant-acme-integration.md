---
name: instant-acme-integration
description: instant-acme 0.8 集成要点(features/自定义根信任/账户凭据存储/finalize 取叶子密钥)与 Pebble 测试服踩坑
metadata:
  type: project
---

ACME 全线接入用 **instant-acme 0.8**(实测 0.8.5),核心集成点(照 api/flows/database/acme.md):

- **Cargo features**:`default-features=false, features=["hyper-rustls","ring","rcgen"]`。
  **必须 ring**(非默认的 aws-lc-rs):全树 rustls/rcgen 已用 ring(见 core Cargo.toml),aws-lc-rs 在
  Windows 需 cmake/nasm(L7 要避)。`cargo tree -i ring/aws-lc-rs` 确认过:加 instant-acme(ring)后
  仍 ring-only,`ClientConfig::builder()` 的 CryptoProvider 确定性选 ring,无 "multiple providers" panic。
  `hyper-rustls`=默认 HTTPS 客户端;`rcgen`=`Order::finalize()` 自动生成叶子密钥+CSR。
- **信任 Pebble 自签 HTTPS**:环境变量 `AUTOHTTPS_ACME_CA_CERT`(PEM 路径)→ `instant_acme::Account::builder_with_root(path)`
  (0.8 提供,读 PEM 文件建自定义 RootCertStore);不设即 `Account::builder()`(系统平台根)。**注册与取证两条路径
  都要用同一 builder**——`from_credentials` 会重新拉 directory(HTTPS),同样需要这个根。见 `services::acme::account_builder()`。
- **账户密钥落盘**:注册成功拿到 `(Account, AccountCredentials)`;`serde_json::to_string(&credentials)` →
  age 密文落盘 → `account_key_ref`。`AccountCredentials` 含 PKCS#8 私钥(`key_pkcs8` 字段),**绝不明文入库**。
  复用:`serde_json::from_slice::<AccountCredentials>` → `builder.from_credentials(creds)`。`account.id()` = CA 账户 kid URL(存 `ca_account_url`)。
- **注册是 202 异步**(非 tasks 任务,acme DEC5):handler 插 `registering` 行 + `tokio::spawn` 后台注册 → 返回 202+detail;
  终态 registered/registration_failed 落库 + 发 SSE `acme_account_status_changed`。curl 验证需 poll `GET /acme/accounts/{id}`。
- **签发流(执行器 run_issue_acme)**:`new_order(NewOrder::new(&[Identifier::Dns(...)])` → `order.authorizations()`
  流式取每域名授权 → `authz.challenge(ChallengeType::Http01)` → `challenge.key_authorization().as_str()` 写
  `<webroot>/.well-known/acme-challenge/<token>` → `challenge.set_ready()` → `order.poll_ready(&RetryPolicy)` 到 Ready
  → `order.finalize()`(**返回叶子私钥 PEM**)→ `order.poll_certificate()`(**返回链 PEM:叶子+中间**)。
  借用纪律:`authorizations()` 借 `&mut order`,必须用 `{ }` 块把授权循环圈起来,块结束才能调 order.poll_ready/finalize;
  循环内先把 token/key_auth/url `.clone()`/`.to_string()` 成 owned 再 `set_ready()`(key_authorization() 是 &self、set_ready() 是 &mut self)。
- **叶子标识/有效期**:CA 返回的是链 PEM 无结构化元数据;用 `ca::parse_leaf_metadata(chain_pem)`(x509-parser 解析**首块**=叶子)
  取 serial/fingerprint/notBefore/notAfter。链整体 age 存为 `cert_pem_ref`,叶子私钥 age 存为 `private_key_ref`。

**Pebble 测试服踩坑**:
- `PEBBLE_VA_ALWAYS_VALID=1` 挑战自动判过,但**仍须走 set_ready**(POST 挑战 URL)把授权从 pending 推到 valid;
  HTTP-01 文件可达性不重要但要放(走流程)。
- **Pebble 随机化证书有效期**:同一账户连开两单,一张签成 90 天、另一张只 6 天(故意防客户端硬编码 90d)。
  副作用:短有效期(<`renewalAdvanceDays`,默认 30)的 acme 证书,下一轮扫描即被判 `expiring_soon` 并触发自动续签(T9);
  **acme 续签执行本切片是桩**→ 证书停在 `renewing`、renew 任务停 `queued`(执行器 tick 对 acme renew/revoke `continue` 跳过)。
  这是预期边界不是 bug(acme 续签/吊销执行留后续切片);实现 acme renew 后自解。
