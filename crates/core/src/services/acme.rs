//! ACME 服务(API acme)—— 账户 / 挑战 list+detail、http01 配置 get/put 为真实读取。
//! 账户注册 / 挑战确认重试等在线交互在 api 层打桩 501。

use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{
    acme_accounts, certificates, challenges, domains, http01_validation_configs, settings, tasks,
};
use crate::services::context::CoreContext;
use crate::services::pagination::{Paged, PageParams};
use crate::services::settings::SINGLETON_ID;
use crate::util::now_rfc3339;
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
