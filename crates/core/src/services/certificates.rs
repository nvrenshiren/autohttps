//! 证书服务(API certificates)—— 全局枢纽。
//!
//! 里程碑1:list / detail / create(全 §2.3 校验)/ delete 为**真实实现**;
//! renew / retry / revoke / export 依赖执行器 + ACME/CA,**在 api 层打桩 501**(见 handlers)。

use crate::domain::enums::{
    AcmeAccountStatus, CertificateStatus, IssuanceMethod, RootCaStatus, TaskStatus, TaskTrigger,
    TaskType, ValidationMethod,
};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{
    acme_accounts, certificate_domains, certificates, domains, root_cas, settings, tasks,
};
use crate::services::context::CoreContext;
use crate::services::pagination::{Paged, PageParams};
use crate::services::settings::SINGLETON_ID;
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;

pub struct DomainRefData {
    pub id: String,
    pub hostname: String,
    pub is_wildcard: bool,
}

pub struct CertRow {
    pub cert: certificates::Model,
    pub domains: Vec<DomainRefData>,
}

pub struct AcmeAccountRefData {
    pub id: String,
    pub ca_label: Option<String>,
    pub environment: Option<String>,
}

pub struct RootCaRefData {
    pub id: String,
    pub name: String,
}

pub struct CertDetailData {
    pub row: CertRow,
    pub acme_account: Option<AcmeAccountRefData>,
    pub root_ca: Option<RootCaRefData>,
    pub active_task_id: Option<String>,
}

#[derive(Default)]
pub struct CertListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub statuses: Vec<CertificateStatus>,
    pub issuance_method: Option<IssuanceMethod>,
    pub domain: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

pub struct IssueCertInput {
    pub issuance_method: IssuanceMethod,
    pub domain_ids: Vec<String>,
    pub acme_account_id: Option<String>,
    pub root_ca_id: Option<String>,
}

async fn san_domains(db: &DatabaseConnection, cert_id: &str) -> CoreResult<Vec<DomainRefData>> {
    let links = certificate_domains::Entity::find()
        .filter(certificate_domains::Column::CertificateId.eq(cert_id))
        .all(db)
        .await?;
    let ids: Vec<String> = links.into_iter().map(|l| l.domain_id).collect();
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let ds = domains::Entity::find()
        .filter(domains::Column::Id.is_in(ids))
        .all(db)
        .await?;
    Ok(ds
        .into_iter()
        .map(|d| DomainRefData { id: d.id, hostname: d.hostname, is_wildcard: d.is_wildcard })
        .collect())
}

async fn build_detail(db: &DatabaseConnection, cert: certificates::Model) -> CoreResult<CertDetailData> {
    let domains = san_domains(db, &cert.id).await?;

    let acme_account = match &cert.acme_account_id {
        Some(aid) => acme_accounts::Entity::find_by_id(aid)
            .one(db)
            .await?
            .map(|a| AcmeAccountRefData { id: a.id, ca_label: a.ca_label, environment: a.environment }),
        None => None,
    };
    let root_ca = match &cert.root_ca_id {
        Some(rid) => root_cas::Entity::find_by_id(rid)
            .one(db)
            .await?
            .map(|r| RootCaRefData { id: r.id, name: r.name }),
        None => None,
    };

    // 当前进行中任务(供进行中态经 tasks 取消)
    let active_task_id = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(&cert.id))
        .filter(tasks::Column::Status.is_in([TaskStatus::Queued, TaskStatus::Running]))
        .order_by_desc(tasks::Column::QueuedAt)
        .one(db)
        .await?
        .map(|t| t.id);

    Ok(CertDetailData {
        row: CertRow { cert, domains },
        acme_account,
        root_ca,
        active_task_id,
    })
}

pub async fn list(ctx: &CoreContext, filter: CertListFilter) -> CoreResult<Paged<CertRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);

    let mut query = certificates::Entity::find();
    if !filter.statuses.is_empty() {
        query = query.filter(certificates::Column::Status.is_in(filter.statuses.clone()));
    }
    if let Some(m) = filter.issuance_method {
        query = query.filter(certificates::Column::IssuanceMethod.eq(m));
    }
    if let Some(h) = filter.domain.filter(|s| !s.is_empty()) {
        // 经 domains(hostname 子串)→ certificate_domains 反查 cert 集
        let domain_ids: Vec<String> = domains::Entity::find()
            .filter(domains::Column::Hostname.contains(h.as_str()))
            .all(db)
            .await?
            .into_iter()
            .map(|d| d.id)
            .collect();
        let cert_ids: Vec<String> = if domain_ids.is_empty() {
            vec![]
        } else {
            certificate_domains::Entity::find()
                .filter(certificate_domains::Column::DomainId.is_in(domain_ids))
                .all(db)
                .await?
                .into_iter()
                .map(|l| l.certificate_id)
                .collect()
        };
        if cert_ids.is_empty() {
            return Ok(Paged { items: vec![], page: page.page, page_size: page.page_size, total: 0 });
        }
        query = query.filter(certificates::Column::Id.is_in(cert_ids));
    }

    let order = matches!(filter.order.as_deref(), Some("desc")).then_some(Order::Desc);
    let (col, default_order) = match filter.sort.as_deref() {
        None | Some("notAfter") => (certificates::Column::NotAfter, Order::Asc),
        Some("createdAt") => (certificates::Column::CreatedAt, Order::Desc),
        Some("updatedAt") => (certificates::Column::UpdatedAt, Order::Desc),
        Some(other) => {
            return Err(CoreError::new(
                ErrorCode::ValidationFailed,
                format!("不支持的排序字段: {other}"),
            ))
        }
    };
    query = query.order_by(col, order.unwrap_or(default_order));

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;

    let mut items = Vec::with_capacity(models.len());
    for cert in models {
        let domains = san_domains(db, &cert.id).await?;
        items.push(CertRow { cert, domains });
    }
    Ok(Paged { items, page: page.page, page_size: page.page_size, total })
}

pub async fn get(ctx: &CoreContext, id: &str) -> CoreResult<CertDetailData> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;
    build_detail(db, cert).await
}

pub async fn create(ctx: &CoreContext, input: IssueCertInput) -> CoreResult<CertDetailData> {
    let db = &ctx.db;

    if input.domain_ids.is_empty() {
        return Err(CoreError::new(ErrorCode::NoDomainsSpecified, "未指定任何域名"));
    }

    // 枢纽 XOR 不变量(_overview §4.1):账户/根 CA 与方式匹配、互斥
    match input.issuance_method {
        IssuanceMethod::Acme if input.root_ca_id.is_some() => {
            return Err(CoreError::new(ErrorCode::IssuanceSourceConflict, "acme 方式不应指定根 CA"))
        }
        IssuanceMethod::SelfSigned if input.acme_account_id.is_some() => {
            return Err(CoreError::new(
                ErrorCode::IssuanceSourceConflict,
                "self_signed 方式不应指定 ACME 账户",
            ))
        }
        _ => {}
    }

    // 域名存在性
    let found = domains::Entity::find()
        .filter(domains::Column::Id.is_in(input.domain_ids.clone()))
        .all(db)
        .await?;
    if found.len() != input.domain_ids.len() {
        let found_ids: std::collections::HashSet<_> = found.iter().map(|d| d.id.as_str()).collect();
        let missing: Vec<&String> =
            input.domain_ids.iter().filter(|id| !found_ids.contains(id.as_str())).collect();
        return Err(CoreError::new(ErrorCode::InvalidDomainReference, "引用了不存在的域名")
            .with_details(serde_json::json!({ "domainIds": missing })));
    }

    // 至多一个通配符(DEC4);通配符须 dns_01
    let wildcard_count = found.iter().filter(|d| d.is_wildcard).count();
    if wildcard_count > 1 {
        return Err(CoreError::new(
            ErrorCode::MultipleWildcardsNotAllowed,
            "一张证书至多包含一个通配符域名",
        ));
    }
    for d in &found {
        if d.is_wildcard && d.validation_method != Some(ValidationMethod::Dns01) {
            return Err(CoreError::new(
                ErrorCode::WildcardRequiresDns01,
                format!("通配符域名 {} 的验证方式必须为 dns_01", d.hostname),
            ));
        }
    }

    // 签发来源前置校验
    let mut acme_account_id = None;
    let mut root_ca_id = None;
    match input.issuance_method {
        IssuanceMethod::Acme => {
            // 账户:显式 or settings 默认
            let account_id = match input.acme_account_id {
                Some(a) => a,
                None => settings::Entity::find_by_id(SINGLETON_ID)
                    .one(db)
                    .await?
                    .and_then(|s| s.default_acme_account_id)
                    .ok_or_else(|| {
                        CoreError::new(ErrorCode::AcmeAccountRequired, "未指定 ACME 账户且无默认账户")
                    })?,
            };
            let account = acme_accounts::Entity::find_by_id(&account_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    CoreError::new(ErrorCode::InvalidAcmeAccountReference, "引用了不存在的 ACME 账户")
                })?;
            if account.status != AcmeAccountStatus::Registered {
                return Err(CoreError::new(
                    ErrorCode::AcmeAccountNotRegistered,
                    "指定的 ACME 账户尚未注册成功",
                ));
            }
            // acme 需按域名验证方式
            for d in &found {
                if d.validation_method.is_none() {
                    return Err(CoreError::new(
                        ErrorCode::DomainValidationMethodRequired,
                        format!("域名 {} 未设置验证方式", d.hostname),
                    ));
                }
            }
            acme_account_id = Some(account_id);
        }
        IssuanceMethod::SelfSigned => {
            let rid = input
                .root_ca_id
                .ok_or_else(|| CoreError::new(ErrorCode::RootCaRequired, "self_signed 需指定根 CA"))?;
            let ca = root_cas::Entity::find_by_id(&rid)
                .one(db)
                .await?
                .ok_or_else(|| {
                    CoreError::new(ErrorCode::InvalidRootCaReference, "引用了不存在的根 CA")
                })?;
            if ca.status != RootCaStatus::Active {
                return Err(CoreError::new(ErrorCode::RootCaExpired, "指定的根 CA 已过期,不可签发"));
            }
            root_ca_id = Some(rid);
        }
    }

    // 创建证书条目(pending_issue,T1)+ SAN 关联 + 入队 issue 任务(TT1)
    let now = now_rfc3339();
    let cert_id = new_id();
    let cert = certificates::ActiveModel {
        id: Set(cert_id.clone()),
        issuance_method: Set(input.issuance_method),
        status: Set(CertificateStatus::PendingIssue),
        acme_account_id: Set(acme_account_id),
        root_ca_id: Set(root_ca_id),
        serial_number: Set(None),
        fingerprint: Set(None),
        not_before: Set(None),
        not_after: Set(None),
        issued_at: Set(None),
        cert_pem_ref: Set(None),
        private_key_ref: Set(None),
        last_error: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    };
    let cert = cert.insert(db).await?;

    for d in &found {
        certificate_domains::ActiveModel {
            certificate_id: Set(cert_id.clone()),
            domain_id: Set(d.id.clone()),
        }
        .insert(db)
        .await?;
    }

    tasks::ActiveModel {
        id: Set(new_id()),
        certificate_id: Set(cert_id.clone()),
        task_type: Set(TaskType::Issue),
        trigger: Set(TaskTrigger::Manual),
        status: Set(TaskStatus::Queued),
        attempt_number: Set(1),
        parent_task_id: Set(None),
        queued_at: Set(now.clone()),
        started_at: Set(None),
        finished_at: Set(None),
        result_summary: Set(None),
        failure_reason: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    // TODO(实现期):任务执行器取出 issue 任务 → 委托 acme/ca → 驱动证书 T2–T4,经 SSE 回推。
    build_detail(db, cert).await
}

/// 吊销(D1,T8/T11/T16 → revoking):校验源态 → 证书转 `revoking` + 入队 `revoke` 任务;
/// 实际作废(记根 CA 本地作废记录 + `revoking→revoked` T18)由执行器完成。202 已受理。
pub async fn revoke(ctx: &CoreContext, id: &str) -> CoreResult<CertDetailData> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    // 适用源态:valid(T8)/ expiring_soon(T11)/ renewal_failed(T16);权威转移表不含 expired
    if !cert.status.can_revoke() {
        return Err(CoreError::new(ErrorCode::InvalidCertState, "当前状态不可吊销").with_details(
            serde_json::json!({ "currentStatus": cert.status, "action": "revoke" }),
        ));
    }

    let now = now_rfc3339();
    // 证书 → revoking(T8/T11/T16)
    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(CertificateStatus::Revoking);
    a.updated_at = Set(now.clone());
    let cert = a.update(db).await?;

    // 入队 revoke 任务(TT1);执行器承接 self_signed 作废
    tasks::ActiveModel {
        id: Set(new_id()),
        certificate_id: Set(cert.id.clone()),
        task_type: Set(TaskType::Revoke),
        trigger: Set(TaskTrigger::Manual),
        status: Set(TaskStatus::Queued),
        attempt_number: Set(1),
        parent_task_id: Set(None),
        queued_at: Set(now.clone()),
        started_at: Set(None),
        finished_at: Set(None),
        result_summary: Set(None),
        failure_reason: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    build_detail(db, cert).await
}

pub async fn delete(ctx: &CoreContext, id: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    if cert.status.is_in_progress() {
        return Err(CoreError::new(
            ErrorCode::CertInProgressCannotDelete,
            "进行中态证书不可删除,请先取消其任务",
        )
        .with_details(serde_json::json!({ "currentStatus": cert.status })));
    }

    let now = now_rfc3339();

    // 未完成任务经清理转 cancelled(trigger=cleanup,§5.5);历史任务只读保留(软引用不级联)
    let unfinished = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(id))
        .filter(tasks::Column::Status.is_in([TaskStatus::Queued, TaskStatus::Running]))
        .all(db)
        .await?;
    for t in unfinished {
        let mut a: tasks::ActiveModel = t.into();
        a.status = Set(TaskStatus::Cancelled);
        a.trigger = Set(TaskTrigger::Cleanup);
        a.finished_at = Set(Some(now.clone()));
        a.result_summary = Set(Some("证书删除,清理未完成任务".into()));
        a.updated_at = Set(now.clone());
        a.update(db).await?;
    }

    // 清除敏感/文件材料(按 *_ref)
    if let Some(r) = &cert.private_key_ref {
        let _ = ctx.secrets.remove(r);
    }
    if let Some(r) = &cert.cert_pem_ref {
        let _ = ctx.secrets.remove(r);
    }

    // 移除 SAN 关联 + 证书条目(certificate_domains.certificate_id CASCADE 兜底)
    certificate_domains::Entity::delete_many()
        .filter(certificate_domains::Column::CertificateId.eq(id))
        .exec(db)
        .await?;
    certificates::Entity::delete_by_id(id).exec(db).await?;
    Ok(())
}
