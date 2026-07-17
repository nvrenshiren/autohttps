//! ACME 服务(API acme)—— 账户 / 挑战 list+detail、http01 配置 get/put 为真实读取。
//! 账户注册(A1)/ 编辑邮箱(A3)/ 注册重试(A4)/ 移除(A5)、DNS-01 挑战确认(B4)/ 重试(B5)
//! 均已去桩;ACME order 机制(建单/挑战推进/finalize/吊销)在 `executor`,本模块委派。
//! `dns-precheck`(B4 可选)用 hickory-resolver 本地查 TXT 预检,已实现(见 [`dns_precheck`])。

use crate::domain::enums::{AcmeAccountStatus, ValidationMethod};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    acme_accounts, certificates, challenges, domains, http01_validation_configs, settings, tasks,
};
use crate::services::context::CoreContext;
use crate::services::pagination::{Paged, PageParams};
use crate::services::settings::SINGLETON_ID;
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;

pub struct AccountRow {
    pub account: acme_accounts::Model,
    pub is_default: bool,
    pub certificate_count: u64,
}

pub struct ChallengeRow {
    pub challenge: challenges::Model,
    pub certificate_id: String,
    pub domain_hostname: Option<String>,
}

#[derive(Default)]
pub struct AccountListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<crate::domain::enums::AcmeAccountStatus>,
}

#[derive(Default)]
pub struct ChallengeListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub task_id: Option<String>,
    pub domain_id: Option<String>,
    pub status: Option<crate::domain::enums::ChallengeStatus>,
    pub certificate_id: Option<String>,
}

async fn default_account_id(db: &DatabaseConnection) -> CoreResult<Option<String>> {
    Ok(settings::Entity::find_by_id(SINGLETON_ID)
        .one(db)
        .await?
        .and_then(|s| s.default_acme_account_id))
}

async fn build_account_row(db: &DatabaseConnection, account: acme_accounts::Model) -> CoreResult<AccountRow> {
    let default_id = default_account_id(db).await?;
    let certificate_count = certificates::Entity::find()
        .filter(certificates::Column::AcmeAccountId.eq(&account.id))
        .count(db)
        .await?;
    Ok(AccountRow {
        is_default: default_id.as_deref() == Some(account.id.as_str()),
        certificate_count,
        account,
    })
}

pub async fn accounts_list(ctx: &CoreContext, filter: AccountListFilter) -> CoreResult<Paged<AccountRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);
    let mut query = acme_accounts::Entity::find();
    if let Some(s) = filter.status {
        query = query.filter(acme_accounts::Column::Status.eq(s));
    }
    query = query.order_by_desc(acme_accounts::Column::CreatedAt);

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;
    let mut items = Vec::with_capacity(models.len());
    for a in models {
        items.push(build_account_row(db, a).await?);
    }
    Ok(Paged { items, page: page.page, page_size: page.page_size, total })
}

pub async fn account_get(ctx: &CoreContext, id: &str) -> CoreResult<AccountRow> {
    let db = &ctx.db;
    let account = acme_accounts::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountNotFound, "ACME 账户不存在"))?;
    build_account_row(db, account).await
}

async fn build_challenge_row(db: &DatabaseConnection, challenge: challenges::Model) -> CoreResult<ChallengeRow> {
    // certificate_id 经 task 关联反查(单一真相)
    let certificate_id = tasks::Entity::find_by_id(&challenge.task_id)
        .one(db)
        .await?
        .map(|t| t.certificate_id)
        .unwrap_or_default();
    let domain_hostname = domains::Entity::find_by_id(&challenge.domain_id)
        .one(db)
        .await?
        .map(|d| d.hostname);
    Ok(ChallengeRow { challenge, certificate_id, domain_hostname })
}

pub async fn challenges_list(ctx: &CoreContext, filter: ChallengeListFilter) -> CoreResult<Paged<ChallengeRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);
    let mut query = challenges::Entity::find();
    if let Some(t) = filter.task_id.filter(|s| !s.is_empty()) {
        query = query.filter(challenges::Column::TaskId.eq(t));
    }
    if let Some(d) = filter.domain_id.filter(|s| !s.is_empty()) {
        query = query.filter(challenges::Column::DomainId.eq(d));
    }
    if let Some(s) = filter.status {
        query = query.filter(challenges::Column::Status.eq(s));
    }
    // certificateId → 经 task 反查其 task 集
    if let Some(cid) = filter.certificate_id.filter(|s| !s.is_empty()) {
        let task_ids: Vec<String> = tasks::Entity::find()
            .filter(tasks::Column::CertificateId.eq(cid))
            .all(db)
            .await?
            .into_iter()
            .map(|t| t.id)
            .collect();
        if task_ids.is_empty() {
            return Ok(Paged { items: vec![], page: page.page, page_size: page.page_size, total: 0 });
        }
        query = query.filter(challenges::Column::TaskId.is_in(task_ids));
    }
    query = query.order_by_desc(challenges::Column::CreatedAt);

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;
    let mut items = Vec::with_capacity(models.len());
    for c in models {
        items.push(build_challenge_row(db, c).await?);
    }
    Ok(Paged { items, page: page.page, page_size: page.page_size, total })
}

pub async fn challenge_get(ctx: &CoreContext, id: &str) -> CoreResult<ChallengeRow> {
    let db = &ctx.db;
    let challenge = challenges::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::ChallengeNotFound, "挑战不存在"))?;
    build_challenge_row(db, challenge).await
}

pub async fn http01_get(ctx: &CoreContext, domain_id: &str) -> CoreResult<http01_validation_configs::Model> {
    http01_validation_configs::Entity::find_by_id(domain_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::Http01ConfigNotFound, "该域名无 HTTP-01 配置"))
}

/// 设置/更新 webroot(按域名 upsert,1:0..1)。
pub async fn http01_put(
    ctx: &CoreContext,
    domain_id: &str,
    webroot_path: String,
) -> CoreResult<http01_validation_configs::Model> {
    let db = &ctx.db;
    if webroot_path.trim().is_empty() {
        return Err(CoreError::validation("webrootPath 不能为空"));
    }
    // 域名须存在(共享规则 domain_not_found)
    if domains::Entity::find_by_id(domain_id).one(db).await?.is_none() {
        return Err(CoreError::new(ErrorCode::DomainNotFound, "域名不存在"));
    }
    let now = now_rfc3339();
    match http01_validation_configs::Entity::find_by_id(domain_id).one(db).await? {
        Some(existing) => {
            let mut a: http01_validation_configs::ActiveModel = existing.into();
            a.webroot_path = Set(webroot_path);
            a.updated_at = Set(now);
            Ok(a.update(db).await?)
        }
        None => {
            let a = http01_validation_configs::ActiveModel {
                domain_id: Set(domain_id.to_string()),
                webroot_path: Set(webroot_path),
                created_at: Set(now.clone()),
                updated_at: Set(now),
            };
            Ok(a.insert(db).await?)
        }
    }
}

// ============ 账户注册(A1 / AT1)+ instant-acme 客户端 ============

/// 注册账户入参(acme api §2.1 `RegisterAcmeAccountRequest`)。
pub struct RegisterAccountInput {
    pub directory_url: String,
    pub ca_label: Option<String>,
    pub contact_email: String,
    pub tos_agreed: bool,
}

/// 构建 instant-acme HTTP 客户端 AccountBuilder。
///
/// 若设 `AUTOHTTPS_ACME_CA_CERT`(PEM 路径),把该根加入客户端信任(测试用,如信任 Pebble 自签 HTTPS);
/// 不设即用系统平台根(生产 Let's Encrypt 等)。
pub fn account_builder() -> CoreResult<instant_acme::AccountBuilder> {
    match std::env::var("AUTOHTTPS_ACME_CA_CERT") {
        Ok(path) if !path.trim().is_empty() => instant_acme::Account::builder_with_root(path.trim())
            .map_err(|e| CoreError::internal(format!("加载 ACME 测试根证书失败: {e}"))),
        _ => instant_acme::Account::builder()
            .map_err(|e| CoreError::internal(format!("初始化 ACME 客户端失败: {e}"))),
    }
}

/// instant-acme 错误 → CoreError(脱敏:错误链不含密钥材料;Problem detail 为 CA 人读原因,可展示)。
pub(crate) fn map_acme_err(e: instant_acme::Error) -> CoreError {
    CoreError::internal(format!("ACME 交互失败: {e}"))
}

/// 由已注册账户行还原 instant-acme `Account`(载入 age 密文凭据,供执行器建单/取证)。
pub async fn load_acme_account(
    ctx: &CoreContext,
    account: &acme_accounts::Model,
) -> CoreResult<instant_acme::Account> {
    let key_ref = account
        .account_key_ref
        .as_deref()
        .ok_or_else(|| CoreError::internal("ACME 账户缺少账户密钥引用"))?;
    let creds_bytes = ctx.secrets.load(key_ref)?;
    let credentials: instant_acme::AccountCredentials = serde_json::from_slice(&creds_bytes)
        .map_err(|e| CoreError::internal(format!("解析 ACME 账户凭据失败: {e}")))?;
    account_builder()?.from_credentials(credentials).await.map_err(map_acme_err)
}

/// 配置并注册账户(A1,AT1)。校验通过 → 插 `registering` 行 + 后台异步向 CA 注册 → 返回该行(202)。
/// 终态(registered/registration_failed)由后台任务落库并经 SSE `acme_account_status_changed` 回推。
pub async fn create_account(ctx: &CoreContext, input: RegisterAccountInput) -> CoreResult<AccountRow> {
    // 校验(acme api §2.1)
    if !input.tos_agreed {
        return Err(CoreError::new(ErrorCode::TosNotAgreed, "注册前须同意服务条款"));
    }
    let directory_url = input.directory_url.trim().to_string();
    if !(directory_url.starts_with("http://") || directory_url.starts_with("https://")) {
        return Err(CoreError::new(ErrorCode::InvalidDirectoryUrl, "ACME 目录 URL 非法"));
    }
    let email = input.contact_email.trim().to_string();
    if !is_valid_email(&email) {
        return Err(CoreError::validation("联系邮箱格式不正确"));
    }

    let now = now_rfc3339();
    let id = new_id();
    let account = acme_accounts::ActiveModel {
        id: Set(id.clone()),
        directory_url: Set(directory_url.clone()),
        ca_label: Set(input.ca_label),
        environment: Set(environment_label(&directory_url)),
        contact_email: Set(email),
        tos_agreed: Set(true),
        status: Set(AcmeAccountStatus::Registering),
        ca_account_url: Set(None),
        account_key_ref: Set(None),
        registered_at: Set(None),
        last_error: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(&ctx.db)
    .await?;
    ctx.emit(DomainEvent::AcmeAccountStatusChanged {
        account_id: id.clone(),
        status: AcmeAccountStatus::Registering,
    });

    // 后台异步向 CA 注册(注册非 tasks 任务,acme DEC5)。
    let ctx2 = ctx.clone();
    tokio::spawn(async move { run_registration(&ctx2, &id).await });

    build_account_row(&ctx.db, account).await
}

/// 后台注册流程:向 CA 注册 → 生成账户密钥密文落盘 → 落终态 + 发 SSE。
async fn run_registration(ctx: &CoreContext, account_id: &str) {
    let outcome = do_register(ctx, account_id).await;
    if let Err(e) = finalize_registration(ctx, account_id, outcome).await {
        tracing::error!(error = %e, account_id, "ACME 账户注册终态落库失败");
    }
}

/// 执行注册,成功返回 (CA 账户 URL, 账户密钥引用键)。
async fn do_register(ctx: &CoreContext, account_id: &str) -> CoreResult<(String, String)> {
    let account = acme_accounts::Entity::find_by_id(account_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountNotFound, "ACME 账户不存在"))?;

    let contact = format!("mailto:{}", account.contact_email);
    let contacts = [contact.as_str()];
    let new_account = instant_acme::NewAccount {
        contact: &contacts,
        terms_of_service_agreed: true,
        only_return_existing: false,
    };
    let (acme_account, credentials) = account_builder()?
        .create(&new_account, account.directory_url.clone(), None)
        .await
        .map_err(map_acme_err)?;

    // 账户凭据(含 PKCS#8 私钥)序列化后 age 密文落盘,库内只存引用(AR4,DB acme §2.2)。
    let creds_json = serde_json::to_string(&credentials)
        .map_err(|e| CoreError::internal(format!("序列化 ACME 账户凭据失败: {e}")))?;
    let key_ref = new_id();
    ctx.secrets.store(&key_ref, creds_json.as_bytes())?;
    Ok((acme_account.id().to_string(), key_ref))
}

/// 落注册终态(AT2/AT3)并发 SSE。
async fn finalize_registration(
    ctx: &CoreContext,
    account_id: &str,
    outcome: CoreResult<(String, String)>,
) -> CoreResult<()> {
    let now = now_rfc3339();
    let (status, model) = match outcome {
        Ok((account_url, key_ref)) => (
            AcmeAccountStatus::Registered,
            acme_accounts::ActiveModel {
                id: Set(account_id.to_string()),
                status: Set(AcmeAccountStatus::Registered),
                ca_account_url: Set(Some(account_url)),
                account_key_ref: Set(Some(key_ref)),
                registered_at: Set(Some(now.clone())),
                last_error: Set(None),
                updated_at: Set(now),
                ..Default::default()
            },
        ),
        Err(e) => (
            AcmeAccountStatus::RegistrationFailed,
            acme_accounts::ActiveModel {
                id: Set(account_id.to_string()),
                status: Set(AcmeAccountStatus::RegistrationFailed),
                last_error: Set(Some(e.message)),
                updated_at: Set(now),
                ..Default::default()
            },
        ),
    };
    model.update(&ctx.db).await?;
    ctx.emit(DomainEvent::AcmeAccountStatusChanged { account_id: account_id.to_string(), status });
    Ok(())
}

// ============ 账户编辑 / 重试 / 移除(A3/A4/A5)============

/// 编辑联系邮箱(A3,跨状态动作)。仅 `registered` 可编辑(否则 account_state_invalid);邮箱格式校验。
/// 本地更新(业务视为同一账户);best-effort 后台向 CA 同步 contact(失败仅日志,不影响结果)。
pub async fn patch_account(
    ctx: &CoreContext,
    id: &str,
    contact_email: String,
) -> CoreResult<AccountRow> {
    let db = &ctx.db;
    let account = acme_accounts::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountNotFound, "ACME 账户不存在"))?;
    if account.status != AcmeAccountStatus::Registered {
        return Err(CoreError::new(ErrorCode::AccountStateInvalid, "仅已注册账户可编辑邮箱"));
    }
    let email = contact_email.trim().to_string();
    if !is_valid_email(&email) {
        return Err(CoreError::validation("联系邮箱格式不正确"));
    }
    let has_key = account.account_key_ref.is_some();
    let mut a: acme_accounts::ActiveModel = account.into();
    a.contact_email = Set(email);
    a.updated_at = Set(now_rfc3339());
    let updated = a.update(db).await?;

    // best-effort:后台向 CA 同步 contact(MVP 不阻塞、失败仅日志;业务仍同一账户,flows §2.3 跨状态动作)
    if has_key {
        let ctx2 = ctx.clone();
        let account_for_ca = updated.clone();
        tokio::spawn(async move {
            let contact = format!("mailto:{}", account_for_ca.contact_email);
            match load_acme_account(&ctx2, &account_for_ca).await {
                Ok(acc) => {
                    if let Err(e) = acc.update_contacts(&[contact.as_str()]).await {
                        tracing::warn!(error = %e, account_id = %account_for_ca.id, "向 CA 同步账户 contact 失败(本地已更新)");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "载入 ACME 账户失败,跳过 CA contact 同步"),
            }
        });
    }
    build_account_row(db, updated).await
}

/// 注册失败重试(A4,AT4)。仅 `registration_failed` → `registering`(202)+ 后台重注册;
/// 其他态 → account_state_invalid。可先 patch 修正邮箱再重试。
pub async fn retry_account(ctx: &CoreContext, id: &str) -> CoreResult<AccountRow> {
    let db = &ctx.db;
    let account = acme_accounts::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountNotFound, "ACME 账户不存在"))?;
    if account.status != AcmeAccountStatus::RegistrationFailed {
        return Err(CoreError::new(ErrorCode::AccountStateInvalid, "仅注册失败的账户可重试"));
    }
    let mut a: acme_accounts::ActiveModel = account.into();
    a.status = Set(AcmeAccountStatus::Registering);
    a.last_error = Set(None);
    a.updated_at = Set(now_rfc3339());
    let updated = a.update(db).await?;
    ctx.emit(DomainEvent::AcmeAccountStatusChanged {
        account_id: id.to_string(),
        status: AcmeAccountStatus::Registering,
    });

    // 后台异步重注册(复用注册流程;终态经 SSE 回推)。
    let ctx2 = ctx.clone();
    let id2 = id.to_string();
    tokio::spawn(async move { run_registration(&ctx2, &id2).await });

    build_account_row(db, updated).await
}

/// 移除账户(A5,AT5)。引用该账户的证书 `acme_account_id` / settings 默认账户置空(SET NULL,
/// 续签需改选账户);清账户密钥材料;删账户行。不存在 → acme_account_not_found。
pub async fn delete_account(ctx: &CoreContext, id: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let account = acme_accounts::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountNotFound, "ACME 账户不存在"))?;

    // 证书引用置空(SET NULL;续签需改选账户)
    let now = now_rfc3339();
    let referencing = certificates::Entity::find()
        .filter(certificates::Column::AcmeAccountId.eq(id))
        .all(db)
        .await?;
    for c in referencing {
        let mut a: certificates::ActiveModel = c.into();
        a.acme_account_id = Set(None);
        a.updated_at = Set(now.clone());
        a.update(db).await?;
    }
    // settings 默认账户指向置空(SET NULL)
    if let Some(s) = settings::Entity::find_by_id(SINGLETON_ID).one(db).await? {
        if s.default_acme_account_id.as_deref() == Some(id) {
            let mut a: settings::ActiveModel = s.into();
            a.default_acme_account_id = Set(None);
            a.updated_at = Set(now.clone());
            a.update(db).await?;
        }
    }
    // 清账户密钥材料(AR4;引用即删)
    if let Some(r) = &account.account_key_ref {
        let _ = ctx.secrets.remove(r);
    }
    acme_accounts::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}

// ============ 挑战确认 / 重试(B4/B5,委派执行器 ACME 机制)============

/// DNS-01 确认已添加 TXT(B4,CT4)→ 通知 CA 校验、全部就绪续推 finalize。返回更新后的挑战详情。
pub async fn confirm_challenge(ctx: &CoreContext, id: &str) -> CoreResult<ChallengeRow> {
    crate::services::executor::confirm_challenge(ctx, id).await?;
    challenge_get(ctx, id).await
}

/// 挑战失败重试(B5,CT7)→ 重建订单取新挑战(委派证书重试;派生新任务)。
pub async fn retry_challenge(ctx: &CoreContext, id: &str) -> CoreResult<()> {
    crate::services::executor::retry_challenge(ctx, id).await
}

// ============ DNS-01 提交前本地预检(B4 可选,acme api §2.3 / flows §4.3)============

/// dns-precheck 结果(acme api §2.3 `{ propagated, observedValues }`)。
pub struct DnsPrecheckOutcome {
    /// 待添加的 `dns_txt_value` 是否已出现在实测 TXT 记录中(即已生效)。
    pub propagated: bool,
    /// 本地解析器实测到的该 TXT 名下的全部记录值(供 UI 对照排障)。
    pub observed_values: Vec<String>,
}

/// DNS-01 提交前本地预检(B4 可选):用系统 DNS 解析器查该挑战 `dns_txt_name`(即 `_acme-challenge.<域名>`)
/// 的 TXT 记录,判断待添加的 `dns_txt_value` 是否已生效。**只读、不改挑战状态**(acme api §2.3)。
///
/// - 仅 DNS-01 挑战适用,否则 `not_dns01_challenge`(422)。
/// - 查询失败(NXDOMAIN / 超时 / 无记录 / 解析器不可用)**不视为挑战失败**,如实返回
///   `propagated=false` + 空 `observed_values`(flows §4.3 本地预检语义;失败仅告警日志,不上抛 HTTP 错误)。
pub async fn dns_precheck(ctx: &CoreContext, id: &str) -> CoreResult<DnsPrecheckOutcome> {
    let challenge = challenges::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::ChallengeNotFound, "挑战不存在"))?;
    if challenge.validation_method != ValidationMethod::Dns01 {
        return Err(CoreError::new(
            ErrorCode::NotDns01Challenge,
            "本地预检仅适用于 DNS-01 挑战",
        ));
    }
    // 查 TXT(名取挑战已算好的 dns_txt_name;为空则无可查 → 视为未生效)。
    let observed = match challenge.dns_txt_name.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() => query_txt_records(name).await,
        _ => Vec::new(),
    };
    let expected = challenge.dns_txt_value.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let propagated = expected.is_some_and(|e| observed.iter().any(|v| v.as_str() == e));
    Ok(DnsPrecheckOutcome { propagated, observed_values: observed })
}

/// 用系统 DNS 解析器查 `name` 的全部 TXT 记录值。**任何失败(初始化 / NXDOMAIN / 超时 / 无记录)
/// 均吞为空 Vec** —— 预检语义下"查不到"即"未生效",不作为 HTTP 错误上抛(acme api §2.3);失败仅日志。
async fn query_txt_records(name: &str) -> Vec<String> {
    let resolver = match hickory_resolver::TokioResolver::builder_tokio() {
        Ok(b) => b.build(),
        Err(e) => {
            tracing::warn!(error = %e, "DNS 预检:初始化系统解析器失败(视为未生效)");
            return Vec::new();
        }
    };
    // 补足末尾点 → 绝对域名,避免解析器追加本地 search 域造成误查。
    let fqdn = if name.ends_with('.') { name.to_string() } else { format!("{name}.") };
    match resolver.txt_lookup(fqdn.clone()).await {
        // 单条 TXT 记录的多个字符串按 wire 顺序拼接为完整值(ACME 值单串,拼接对其无影响)。
        Ok(lookup) => lookup
            .iter()
            .map(|txt| {
                let bytes: Vec<u8> = txt.txt_data().iter().flat_map(|s| s.iter().copied()).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            })
            .collect(),
        Err(e) => {
            tracing::debug!(name = %fqdn, error = %e, "DNS 预检:未查到 TXT(NXDOMAIN/超时/无记录,视为未生效)");
            Vec::new()
        }
    }
}

/// MVP 最小邮箱校验:恰含一个 `@`,本地/域部分非空,域含 `.` 且首尾非 `.`。
fn is_valid_email(email: &str) -> bool {
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}

/// 由目录 URL 推展示用环境标签(DB acme §2:environment 为展示属性、由 directory_url 决定,非 §4.3 枚举)。
fn environment_label(directory_url: &str) -> Option<String> {
    let u = directory_url.to_ascii_lowercase();
    let is_test = ["staging", "test", "pebble", "localhost", "127.0.0.1"]
        .iter()
        .any(|m| u.contains(m));
    Some(if is_test { "测试".to_string() } else { "生产".to_string() })
}
