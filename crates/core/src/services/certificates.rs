//! 证书服务(API certificates)—— 全局枢纽。
//!
//! list / detail / create(全 §2.3 校验)/ delete / revoke / renew / retry / export 为**真实实现**;
//! 异步执行(签发/续签/吊销)委托执行器(self_signed 已接入,acme 待后续)。取消经 tasks 端点,
//! 驱动本模块 `rollback_on_cancel`(T21–T24)。

use crate::domain::enums::{
    AcmeAccountStatus, CertificateStatus, IssuanceMethod, RootCaStatus, TaskStatus, TaskTrigger,
    TaskType, ValidationMethod,
};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    acme_accounts, certificate_domains, certificates, domains, internal_cert_revocations, root_cas,
    settings, tasks,
};
use crate::services::context::CoreContext;
use crate::services::dashboard;
use crate::services::pagination::{PageParams, Paged};
use crate::services::settings::SINGLETON_ID;
use crate::util::{days_until, new_id, now_rfc3339};
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

/// 入队一个任务(TT1)并发 `task_status_changed`(queued)。触发方式由调用方给出:operator 发起的
/// 签发/续签/吊销/重试用 `manual`;扫描器自动续签用 `auto`(SF2);清理用 `cleanup`(见 delete)。
async fn enqueue_task(
    ctx: &CoreContext,
    cert_id: &str,
    task_type: TaskType,
    trigger: TaskTrigger,
    parent_task_id: Option<String>,
    attempt_number: i32,
    now: &str,
) -> CoreResult<()> {
    let task_id = new_id();
    tasks::ActiveModel {
        id: Set(task_id.clone()),
        certificate_id: Set(cert_id.to_string()),
        task_type: Set(task_type),
        trigger: Set(trigger),
        status: Set(TaskStatus::Queued),
        attempt_number: Set(attempt_number),
        parent_task_id: Set(parent_task_id),
        queued_at: Set(now.to_string()),
        started_at: Set(None),
        finished_at: Set(None),
        result_summary: Set(None),
        failure_reason: Set(None),
        created_at: Set(now.to_string()),
        updated_at: Set(now.to_string()),
    }
    .insert(&ctx.db)
    .await?;
    ctx.emit(DomainEvent::TaskStatusChanged {
        task_id,
        certificate_id: cert_id.to_string(),
        status: TaskStatus::Queued,
    });
    Ok(())
}

/// 续签/取消回退所需的续签提前天数(settings SF1);无 settings 行时取默认 30。
async fn renewal_advance_days(db: &DatabaseConnection) -> CoreResult<i64> {
    Ok(settings::Entity::find_by_id(SINGLETON_ID)
        .one(db)
        .await?
        .map(|s| s.renewal_advance_days as i64)
        .unwrap_or(30))
}

/// 该证书当前序列号是否已被记作废(self_signed 本地作废记录;判"续签源为已吊销")。
async fn is_current_serial_revoked(
    db: &DatabaseConnection,
    cert: &certificates::Model,
) -> CoreResult<bool> {
    let Some(serial) = cert.serial_number.as_deref() else {
        return Ok(false);
    };
    let n = internal_cert_revocations::Entity::find()
        .filter(internal_cert_revocations::Column::CertificateId.eq(&cert.id))
        .filter(internal_cert_revocations::Column::SerialNumber.eq(serial))
        .count(db)
        .await?;
    Ok(n > 0)
}

/// 由证书文件/有效期/作废记录推断其"发起前"的稳态(取消回退用)。
/// 已作废 → revoked;否则按有效期:已过期 → expired、进入续签窗口 → expiring_soon、其余 → valid。
fn natural_status(
    not_after: Option<&str>,
    advance_days: i64,
    is_revoked: bool,
) -> CertificateStatus {
    if is_revoked {
        return CertificateStatus::Revoked;
    }
    match days_until(not_after) {
        None => CertificateStatus::Valid, // 有文件但无 notAfter(理论不达),保守取 valid
        Some(d) if d < 0 => CertificateStatus::Expired,
        Some(d) if d <= advance_days => CertificateStatus::ExpiringSoon,
        Some(_) => CertificateStatus::Valid,
    }
}

/// 签发来源前置校验(renew/retry 共用):self_signed 根 CA 仍 active、acme 账户仍 registered。
/// 账户被移除置空 → 提示改选(acme_account_required)。
async fn validate_issuance_source(
    db: &DatabaseConnection,
    cert: &certificates::Model,
) -> CoreResult<()> {
    match cert.issuance_method {
        IssuanceMethod::SelfSigned => {
            let rid = cert
                .root_ca_id
                .as_deref()
                .ok_or_else(|| CoreError::internal("self_signed 证书缺少根 CA 引用"))?;
            let ca = root_cas::Entity::find_by_id(rid)
                .one(db)
                .await?
                .ok_or_else(|| CoreError::new(ErrorCode::InvalidRootCaReference, "根 CA 不存在"))?;
            if ca.status != RootCaStatus::Active {
                return Err(CoreError::new(
                    ErrorCode::RootCaExpired,
                    "指定的根 CA 已过期,不可签发",
                ));
            }
        }
        IssuanceMethod::Acme => {
            let aid = cert.acme_account_id.as_deref().ok_or_else(|| {
                CoreError::new(
                    ErrorCode::AcmeAccountRequired,
                    "关联 ACME 账户已被移除,请改选账户",
                )
            })?;
            let acc = acme_accounts::Entity::find_by_id(aid)
                .one(db)
                .await?
                .ok_or_else(|| {
                    CoreError::new(
                        ErrorCode::InvalidAcmeAccountReference,
                        "引用了不存在的 ACME 账户",
                    )
                })?;
            if acc.status != AcmeAccountStatus::Registered {
                return Err(CoreError::new(
                    ErrorCode::AcmeAccountNotRegistered,
                    "指定的 ACME 账户尚未注册成功",
                ));
            }
        }
    }
    Ok(())
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
        .map(|d| DomainRefData {
            id: d.id,
            hostname: d.hostname,
            is_wildcard: d.is_wildcard,
        })
        .collect())
}

async fn build_detail(
    db: &DatabaseConnection,
    cert: certificates::Model,
) -> CoreResult<CertDetailData> {
    let domains = san_domains(db, &cert.id).await?;

    let acme_account = match &cert.acme_account_id {
        Some(aid) => acme_accounts::Entity::find_by_id(aid)
            .one(db)
            .await?
            .map(|a| AcmeAccountRefData {
                id: a.id,
                ca_label: a.ca_label,
                environment: a.environment,
            }),
        None => None,
    };
    let root_ca = match &cert.root_ca_id {
        Some(rid) => root_cas::Entity::find_by_id(rid)
            .one(db)
            .await?
            .map(|r| RootCaRefData {
                id: r.id,
                name: r.name,
            }),
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
            return Ok(Paged {
                items: vec![],
                page: page.page,
                page_size: page.page_size,
                total: 0,
            });
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
    Ok(Paged {
        items,
        page: page.page,
        page_size: page.page_size,
        total,
    })
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
        return Err(CoreError::new(
            ErrorCode::NoDomainsSpecified,
            "未指定任何域名",
        ));
    }

    // 枢纽 XOR 不变量(_overview §4.1):账户/根 CA 与方式匹配、互斥
    match input.issuance_method {
        IssuanceMethod::Acme if input.root_ca_id.is_some() => {
            return Err(CoreError::new(
                ErrorCode::IssuanceSourceConflict,
                "acme 方式不应指定根 CA",
            ))
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
        let missing: Vec<&String> = input
            .domain_ids
            .iter()
            .filter(|id| !found_ids.contains(id.as_str()))
            .collect();
        return Err(
            CoreError::new(ErrorCode::InvalidDomainReference, "引用了不存在的域名")
                .with_details(serde_json::json!({ "domainIds": missing })),
        );
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
                        CoreError::new(
                            ErrorCode::AcmeAccountRequired,
                            "未指定 ACME 账户且无默认账户",
                        )
                    })?,
            };
            let account = acme_accounts::Entity::find_by_id(&account_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    CoreError::new(
                        ErrorCode::InvalidAcmeAccountReference,
                        "引用了不存在的 ACME 账户",
                    )
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
            let rid = input.root_ca_id.ok_or_else(|| {
                CoreError::new(ErrorCode::RootCaRequired, "self_signed 需指定根 CA")
            })?;
            let ca = root_cas::Entity::find_by_id(&rid)
                .one(db)
                .await?
                .ok_or_else(|| {
                    CoreError::new(ErrorCode::InvalidRootCaReference, "引用了不存在的根 CA")
                })?;
            if ca.status != RootCaStatus::Active {
                return Err(CoreError::new(
                    ErrorCode::RootCaExpired,
                    "指定的根 CA 已过期,不可签发",
                ));
            }
            root_ca_id = Some(rid);
        }
    }

    // 创建证书条目(pending_issue,T1)+ SAN 关联(事务:任一步失败整体回滚,不留半建条目)
    let now = now_rfc3339();
    let cert_id = new_id();
    let txn = db.begin().await?;
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
    let cert = cert.insert(&txn).await?;
    for d in &found {
        certificate_domains::ActiveModel {
            certificate_id: Set(cert_id.clone()),
            domain_id: Set(d.id.clone()),
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    emit_cert(ctx, &cert_id, CertificateStatus::PendingIssue);

    // 入队 issue 任务(TT1);执行器承接 self_signed 签发 → 驱动证书 T2–T4(acme 待后续)。
    enqueue_task(
        ctx,
        &cert_id,
        TaskType::Issue,
        TaskTrigger::Manual,
        None,
        1,
        &now,
    )
    .await?;
    dashboard::emit_changed(ctx).await;

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
        return Err(
            CoreError::new(ErrorCode::InvalidCertState, "当前状态不可吊销").with_details(
                serde_json::json!({ "currentStatus": cert.status, "action": "revoke" }),
            ),
        );
    }

    let now = now_rfc3339();
    // 证书 → revoking(T8/T11/T16)
    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(CertificateStatus::Revoking);
    a.updated_at = Set(now.clone());
    let cert = a.update(db).await?;
    emit_cert(ctx, &cert.id, CertificateStatus::Revoking);

    // 入队 revoke 任务(TT1);执行器承接 self_signed 作废
    enqueue_task(
        ctx,
        &cert.id,
        TaskType::Revoke,
        TaskTrigger::Manual,
        None,
        1,
        &now,
    )
    .await?;
    dashboard::emit_changed(ctx).await;

    build_detail(db, cert).await
}

/// 续签 / 再获取(C1,§2.3 源态门控)→ 证书转 `renewing` + 入队 `renew` 任务;
/// 执行器承接 self_signed 重签(经原根 CA、刷新同一行 serial/有效期,T12→valid,不新建实体 DC1)。202。
pub async fn renew(ctx: &CoreContext, id: &str) -> CoreResult<CertDetailData> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    // 适用源态:valid(T7)/ expiring_soon(T9)/ expired(T17)/ revoked(T20)
    if !cert.status.can_renew() {
        return Err(
            CoreError::new(ErrorCode::InvalidCertState, "当前状态不可续签").with_details(
                serde_json::json!({ "currentStatus": cert.status, "action": "renew" }),
            ),
        );
    }
    // 来源前置(self_signed 根 CA 仍 active / acme 账户仍 registered)
    validate_issuance_source(db, &cert).await?;

    let now = now_rfc3339();
    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(CertificateStatus::Renewing);
    a.last_error = Set(None);
    a.updated_at = Set(now.clone());
    let cert = a.update(db).await?;
    emit_cert(ctx, &cert.id, CertificateStatus::Renewing);

    enqueue_task(
        ctx,
        &cert.id,
        TaskType::Renew,
        TaskTrigger::Manual,
        None,
        1,
        &now,
    )
    .await?;
    dashboard::emit_changed(ctx).await;
    build_detail(db, cert).await
}

/// 失败重试(B2·C3):issue_failed→T5(派生 issue)/ renewal_failed→T14(派生 renew)。
/// 派生新任务(TT7,parent=最近同类失败任务、attempt+1),证书回进行中态。202。
pub async fn retry(ctx: &CoreContext, id: &str) -> CoreResult<CertDetailData> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    // 适用源态:issue_failed(T5)/ renewal_failed(T14)
    if !cert.status.can_retry() {
        return Err(
            CoreError::new(ErrorCode::InvalidCertState, "当前状态不可重试").with_details(
                serde_json::json!({ "currentStatus": cert.status, "action": "retry" }),
            ),
        );
    }
    let task_type = match cert.status {
        CertificateStatus::IssueFailed => TaskType::Issue,
        CertificateStatus::RenewalFailed => TaskType::Renew,
        _ => unreachable!("can_retry 已门控"),
    };
    validate_issuance_source(db, &cert).await?;

    // 取最近同类失败任务作重试链父;无则新链(parent=None, attempt=1)
    let parent = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(&cert.id))
        .filter(tasks::Column::Status.eq(TaskStatus::Failed))
        .filter(tasks::Column::TaskType.eq(task_type))
        .order_by_desc(tasks::Column::QueuedAt)
        .one(db)
        .await?;

    apply_retry(ctx, &cert, task_type, parent.as_ref()).await?;
    dashboard::emit_changed(ctx).await;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;
    build_detail(db, cert).await
}

/// 由 tasks 端点(POST /tasks/{id}/retry)派生重试:对指定失败任务派生同类型新任务。
/// 与 certificates::retry 收敛于同一 core 操作(重试双入口,tasks §4)。
pub async fn derive_retry_from_task(
    ctx: &CoreContext,
    cert: &certificates::Model,
    failed_task: &tasks::Model,
) -> CoreResult<()> {
    apply_retry(ctx, cert, failed_task.task_type, Some(failed_task)).await
}

/// 重试派生核心:置证书进行中态(issue→issuing T5 / renew→renewing T14 / revoke→revoking)、
/// 清 last_error、派生新任务入队。签名类任务(issue/renew)先校验来源。
async fn apply_retry(
    ctx: &CoreContext,
    cert: &certificates::Model,
    task_type: TaskType,
    parent: Option<&tasks::Model>,
) -> CoreResult<()> {
    let db = &ctx.db;
    if matches!(task_type, TaskType::Issue | TaskType::Renew) {
        validate_issuance_source(db, cert).await?;
    }
    let in_progress = match task_type {
        TaskType::Issue => CertificateStatus::Issuing,
        TaskType::Renew => CertificateStatus::Renewing,
        TaskType::Revoke => CertificateStatus::Revoking,
    };
    let (parent_id, attempt) = match parent {
        Some(p) => (Some(p.id.clone()), p.attempt_number + 1),
        None => (None, 1),
    };
    let now = now_rfc3339();
    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(in_progress);
    a.last_error = Set(None);
    a.updated_at = Set(now.clone());
    a.update(db).await?;
    emit_cert(ctx, &cert.id, in_progress);
    enqueue_task(
        ctx,
        &cert.id,
        task_type,
        TaskTrigger::Manual,
        parent_id,
        attempt,
        &now,
    )
    .await?;
    Ok(())
}

/// 扫描器自动续签(T9 / SF2:`trigger=auto`)。来源校验失败(self_signed 根 CA 非 active /
/// acme 账户非 registered)则**跳过**(返回 `false`,避免失败循环空转);否则置证书 `renewing`
/// + 入队 auto `renew` 任务。执行器承接重签(self_signed 直接重签;acme HTTP-01 自动完成、DNS-01 挂起于 `awaiting_manual` 等用户 confirm)。
pub async fn auto_renew(ctx: &CoreContext, cert: &certificates::Model) -> CoreResult<bool> {
    if validate_issuance_source(&ctx.db, cert).await.is_err() {
        return Ok(false);
    }
    let now = now_rfc3339();
    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(CertificateStatus::Renewing);
    a.last_error = Set(None);
    a.updated_at = Set(now.clone());
    a.update(&ctx.db).await?;
    emit_cert(ctx, &cert.id, CertificateStatus::Renewing);
    enqueue_task(
        ctx,
        &cert.id,
        TaskType::Renew,
        TaskTrigger::Auto,
        None,
        1,
        &now,
    )
    .await?;
    Ok(true)
}

/// 取消 → 证书回退(T21–T24,证书状态机唯一真相;tasks 侧只触发)。由 tasks::cancel_task 调用。
/// - issue 取消 → issue_failed(T21 待签发 / T22 签发中);
/// - renew 取消 → 有 parent(retry 源)则 renewal_failed(T14 源),否则按有效期/作废推断发起前态(T23);
/// - revoke 取消 → 按有效期推断发起前态(T24,效果同 T19);证书未变。
/// - 竞态防护:仅当证书仍处该任务对应的进行中态才回退(执行器可能已推进为终态)。
pub async fn rollback_on_cancel(ctx: &CoreContext, task: &tasks::Model) -> CoreResult<()> {
    let db = &ctx.db;
    let Some(cert) = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?
    else {
        return Ok(()); // 证书已删(清理路径);无回退对象
    };

    // 该任务对应的"进行中态"匹配校验(防竞态覆盖执行器已推进的终态)
    let expected = match task.task_type {
        TaskType::Issue => {
            matches!(
                cert.status,
                CertificateStatus::PendingIssue | CertificateStatus::Issuing
            )
        }
        TaskType::Renew => matches!(cert.status, CertificateStatus::Renewing),
        TaskType::Revoke => matches!(cert.status, CertificateStatus::Revoking),
    };
    if !expected {
        return Ok(());
    }

    let new_status = match task.task_type {
        TaskType::Issue => CertificateStatus::IssueFailed, // T21/T22
        TaskType::Renew => {
            if task.parent_task_id.is_some() {
                CertificateStatus::RenewalFailed // T14 源(retry)
            } else {
                let adv = renewal_advance_days(db).await?;
                let revoked = is_current_serial_revoked(db, &cert).await?;
                natural_status(cert.not_after.as_deref(), adv, revoked)
            }
        }
        TaskType::Revoke => {
            let adv = renewal_advance_days(db).await?;
            let base = natural_status(cert.not_after.as_deref(), adv, false);
            // 源为 renewal_failed(T16)近似恢复:仍有旧失败摘要且当前有效则回 renewal_failed
            if cert.last_error.is_some()
                && matches!(
                    base,
                    CertificateStatus::Valid | CertificateStatus::ExpiringSoon
                )
            {
                CertificateStatus::RenewalFailed
            } else {
                base
            }
        }
    };

    let mut a: certificates::ActiveModel = cert.clone().into();
    a.status = Set(new_status);
    if matches!(task.task_type, TaskType::Issue) {
        a.last_error = Set(Some("首签任务已取消".to_string()));
    }
    a.updated_at = Set(now_rfc3339());
    a.update(db).await?;
    emit_cert(ctx, &cert.id, new_status);
    Ok(())
}

/// 发 `certificate_status_changed`(证书状态机唯一真相在 core,事件仅为失效信号,不搬实体)。
fn emit_cert(ctx: &CoreContext, cert_id: &str, status: CertificateStatus) {
    ctx.emit(DomainEvent::CertificateStatusChanged {
        certificate_id: cert_id.to_string(),
        status,
    });
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

    // 事务:取消未完成任务 + 移除 SAN 关联 + 删证书行,三步同成败(失败整体回滚,不留半删状态)。
    // 事件与密钥文件清理放到提交之后(广播/文件系统不参与事务)。
    let txn = db.begin().await?;
    let mut cancelled: Vec<(String, String)> = Vec::new();
    // 未完成任务经清理转 cancelled(trigger=cleanup,§5.5);历史任务只读保留(软引用不级联)
    let unfinished = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(id))
        .filter(tasks::Column::Status.is_in([TaskStatus::Queued, TaskStatus::Running]))
        .all(&txn)
        .await?;
    for t in unfinished {
        cancelled.push((t.id.clone(), t.certificate_id.clone()));
        let mut a: tasks::ActiveModel = t.into();
        a.status = Set(TaskStatus::Cancelled);
        a.trigger = Set(TaskTrigger::Cleanup);
        a.finished_at = Set(Some(now.clone()));
        a.result_summary = Set(Some("证书删除,清理未完成任务".into()));
        a.updated_at = Set(now.clone());
        a.update(&txn).await?;
    }

    // 移除 SAN 关联 + 证书条目(certificate_domains.certificate_id CASCADE 兜底)
    certificate_domains::Entity::delete_many()
        .filter(certificate_domains::Column::CertificateId.eq(id))
        .exec(&txn)
        .await?;
    certificates::Entity::delete_by_id(id).exec(&txn).await?;
    txn.commit().await?;

    // 提交后:发取消事件 + 清除敏感/文件材料(按 *_ref;失败 best-effort,孤儿由 boot 清扫兜底)
    for (tid, cid) in cancelled {
        ctx.emit(DomainEvent::TaskStatusChanged {
            task_id: tid,
            certificate_id: cid,
            status: TaskStatus::Cancelled,
        });
    }
    if let Some(r) = &cert.private_key_ref {
        let _ = ctx.secrets.remove(r);
    }
    if let Some(r) = &cert.cert_pem_ref {
        let _ = ctx.secrets.remove(r);
    }
    // 删除移除了一张证书 → 待处理集合可能变动,发红点合并信号。
    dashboard::emit_changed(ctx).await;
    Ok(())
}

// ============ 导出(E1,§2.8)============

/// 导出内容(§2.8 `parts`)。`private_key` 是唯一读密钥出口(须 acknowledge)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPart {
    Leaf,
    Chain,
    Fullchain,
    PrivateKey,
}

impl ExportPart {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "leaf" => Some(Self::Leaf),
            "chain" => Some(Self::Chain),
            "fullchain" => Some(Self::Fullchain),
            "private_key" => Some(Self::PrivateKey),
            _ => None,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Leaf => "leaf",
            Self::Chain => "chain",
            Self::Fullchain => "fullchain",
            Self::PrivateKey => "private_key",
        }
    }
}

pub struct ExportInput {
    pub parts: Vec<ExportPart>,
    pub acknowledge_key_export: bool,
}

pub struct ExportBundle {
    pub pem: String,
    pub filename: String,
}

fn push_pem(out: &mut String, pem: &str) {
    let trimmed = pem.trim();
    if trimmed.is_empty() {
        return;
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(trimmed);
    out.push('\n');
}

/// 导出证书材料(E1)。读 `cert_pem_ref`/`private_key_ref` → age 解密 → 拼 PEM(二进制下载)。
/// - 无文件态(未签发/签发中/签发失败,或缺 `cert_pem_ref`)→ `409 cert_not_exportable`;
/// - 含 `private_key` 未确认 → `422 key_export_not_acknowledged`;
/// - 私钥导出是**唯一读密钥出口**(DTO 仍绝不含 ref)。self_signed 的 `chain` 取其根 CA 证书。
pub async fn export(ctx: &CoreContext, id: &str, input: ExportInput) -> CoreResult<ExportBundle> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    // 无文件态不可导出(§2.8:pending_issue/issuing/issue_failed 或缺文件引用)
    let Some(cert_ref) = cert.cert_pem_ref.as_deref() else {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    };
    if !cert.status.is_exportable() {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    }

    let wants_key = input
        .parts
        .iter()
        .any(|p| matches!(p, ExportPart::PrivateKey));
    if wants_key && !input.acknowledge_key_export {
        return Err(CoreError::new(
            ErrorCode::KeyExportNotAcknowledged,
            "导出私钥需确认敏感数据风险(acknowledgeKeyExport=true)",
        ));
    }

    // 叶子证书(公开材料,age 解密)
    let leaf_pem = String::from_utf8(ctx.secrets.load(cert_ref)?)
        .map_err(|_| CoreError::internal("证书文件材料损坏"))?;

    // 链:self_signed 取根 CA 证书;acme 的中间链在其 fullchain 落地(待后续)
    let chain_pem = match cert.root_ca_id.as_deref() {
        Some(rid) => root_cas::Entity::find_by_id(rid)
            .one(db)
            .await?
            .map(|r| r.cert_pem)
            .unwrap_or_default(),
        None => String::new(),
    };

    // 私钥(敏感 AR4,仅此出口;须已确认)
    let key_pem = if wants_key {
        let key_ref = cert.private_key_ref.as_deref().ok_or_else(|| {
            CoreError::new(ErrorCode::CertNotExportable, "证书无私钥文件,不可导出")
        })?;
        String::from_utf8(ctx.secrets.load(key_ref)?)
            .map_err(|_| CoreError::internal("私钥材料损坏"))?
    } else {
        String::new()
    };

    // 按请求顺序拼接(去重),各 part 之间以换行分隔
    let mut out = String::new();
    let mut seen: Vec<ExportPart> = Vec::new();
    let mut labels: Vec<&str> = Vec::new();
    for p in &input.parts {
        if seen.contains(p) {
            continue;
        }
        seen.push(*p);
        labels.push(p.label());
        match p {
            ExportPart::Leaf => push_pem(&mut out, &leaf_pem),
            ExportPart::Chain => push_pem(&mut out, &chain_pem),
            ExportPart::Fullchain => {
                push_pem(&mut out, &leaf_pem);
                push_pem(&mut out, &chain_pem);
            }
            ExportPart::PrivateKey => push_pem(&mut out, &key_pem),
        }
    }

    let filename = format!("certificate-{id}-{}.pem", labels.join("-"));
    Ok(ExportBundle { pem: out, filename })
}

// ============ 部署目标导出(§2.8 扩展;zip 打包在 api 层)============

/// 部署目标。zip 内文件组织按目标服务的部署格式;全部含私钥(须 acknowledge)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    /// fullchain.pem + privkey.pem
    Nginx,
    /// cert.pem + chain.pem + privkey.pem
    Apache,
    /// 单文件 .pfx(PKCS#12,口令必填)
    Iis,
    /// 单文件 .pem(叶子 + 私钥 + 链,HAProxy 合并格式)
    Haproxy,
}

impl ExportTarget {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "nginx" => Some(Self::Nginx),
            "apache" => Some(Self::Apache),
            "iis" => Some(Self::Iis),
            "haproxy" => Some(Self::Haproxy),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "apache",
            Self::Iis => "iis",
            Self::Haproxy => "haproxy",
        }
    }
}

pub struct ExportFile {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct TargetBundle {
    pub files: Vec<ExportFile>,
    pub zip_name: String,
}

/// 供命名用的主域名(净化非法字符),无域名回退证书 id。
fn export_stem(domains: &[DomainRefData], id: &str) -> String {
    domains
        .first()
        .map(|d| {
            d.hostname.replace(
                |c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'),
                "_",
            )
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("certificate-{id}"))
}

/// PEM → DER(单块 / 多块),材料损坏 → internal。
fn pem_to_der_many(pem_str: &str) -> CoreResult<Vec<Vec<u8>>> {
    pem::parse_many(pem_str)
        .map(|ps| ps.into_iter().map(|p| p.into_contents()).collect())
        .map_err(|_| CoreError::internal("证书材料 PEM 解析失败"))
}

/// PFX(PKCS#12)打包:叶子在前、链在后;3DES 加密以兼容 IIS / Windows 导入。
fn build_pfx(
    stem: &str,
    leaf_pem: &str,
    chain_pem: &str,
    key_pem: &str,
    password: &str,
) -> CoreResult<Vec<u8>> {
    use p12_keystore::{
        Certificate, EncryptionAlgorithm, KeyStore, KeyStoreEntry, PrivateKeyChain,
    };

    let key_der = pem::parse(key_pem)
        .map(|p| p.into_contents())
        .map_err(|_| CoreError::internal("私钥材料 PEM 解析失败"))?;
    let mut certs = Vec::new();
    for der in pem_to_der_many(leaf_pem)? {
        certs.push(
            Certificate::from_der(&der)
                .map_err(|_| CoreError::internal("叶子证书 DER 解析失败"))?,
        );
    }
    for der in pem_to_der_many(chain_pem)? {
        certs.push(
            Certificate::from_der(&der).map_err(|_| CoreError::internal("链证书 DER 解析失败"))?,
        );
    }

    let mut ks = KeyStore::new();
    ks.add_entry(
        stem,
        KeyStoreEntry::PrivateKeyChain(PrivateKeyChain::new(key_der, b"1", certs)),
    );
    ks.writer(password)
        .encryption_algorithm(EncryptionAlgorithm::PbeWithShaAnd3KeyTripleDesCbc)
        .write()
        .map_err(|_| CoreError::internal("PFX 打包失败"))
}

/// 按部署目标导出文件组(E1 扩展)。与 `export` 同规则:无文件态 → 409;全部目标含私钥。
/// `iis` 目标 `pfx_password` 必填(前端已校验,服务层兜底)。
pub async fn export_target(
    ctx: &CoreContext,
    id: &str,
    target: ExportTarget,
    pfx_password: Option<&str>,
) -> CoreResult<TargetBundle> {
    let db = &ctx.db;
    let cert = certificates::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "证书不存在"))?;

    let Some(cert_ref) = cert.cert_pem_ref.as_deref() else {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    };
    if !cert.status.is_exportable() {
        return Err(CoreError::new(
            ErrorCode::CertNotExportable,
            "证书尚无本地文件,不可导出",
        ));
    }

    let leaf_pem = String::from_utf8(ctx.secrets.load(cert_ref)?)
        .map_err(|_| CoreError::internal("证书文件材料损坏"))?;
    let chain_pem = match cert.root_ca_id.as_deref() {
        Some(rid) => root_cas::Entity::find_by_id(rid)
            .one(db)
            .await?
            .map(|r| r.cert_pem)
            .unwrap_or_default(),
        None => String::new(),
    };
    let key_ref = cert
        .private_key_ref
        .as_deref()
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotExportable, "证书无私钥文件,不可导出"))?;
    let key_pem = String::from_utf8(ctx.secrets.load(key_ref)?)
        .map_err(|_| CoreError::internal("私钥材料损坏"))?;

    let stem = export_stem(&san_domains(db, id).await?, id);
    let pem_of = |parts: &[&str]| -> Vec<u8> {
        let mut s = String::new();
        for p in parts {
            push_pem(&mut s, p);
        }
        s.into_bytes()
    };

    let files: Vec<ExportFile> = match target {
        ExportTarget::Nginx => vec![
            ExportFile {
                name: format!("{stem}.fullchain.pem"),
                data: pem_of(&[&leaf_pem, &chain_pem]),
            },
            ExportFile {
                name: format!("{stem}.privkey.pem"),
                data: pem_of(&[&key_pem]),
            },
        ],
        ExportTarget::Apache => vec![
            ExportFile {
                name: format!("{stem}.cert.pem"),
                data: pem_of(&[&leaf_pem]),
            },
            ExportFile {
                name: format!("{stem}.chain.pem"),
                data: pem_of(&[&chain_pem]),
            },
            ExportFile {
                name: format!("{stem}.privkey.pem"),
                data: pem_of(&[&key_pem]),
            },
        ],
        ExportTarget::Haproxy => vec![ExportFile {
            name: format!("{stem}.pem"),
            data: pem_of(&[&leaf_pem, &key_pem, &chain_pem]),
        }],
        ExportTarget::Iis => {
            let password = pfx_password
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| CoreError::new(ErrorCode::ValidationFailed, "PFX 导出口令必填"))?;
            vec![ExportFile {
                name: format!("{stem}.pfx"),
                data: build_pfx(&stem, &leaf_pem, &chain_pem, &key_pem, password)?,
            }]
        }
    };

    Ok(TargetBundle {
        files,
        zip_name: format!("{stem}-{}.zip", target.label()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 生成自签叶子材料(leaf/key PEM)供 PFX 打包测试。
    fn self_signed_materials() -> (String, String) {
        let key = rcgen::KeyPair::generate().expect("keypair");
        let params =
            rcgen::CertificateParams::new(vec!["example.com".to_string()]).expect("params");
        let cert = params.self_signed(&key).expect("self-signed");
        (cert.pem(), key.serialize_pem())
    }

    #[test]
    fn export_stem_sanitizes_hostname() {
        let ds = vec![DomainRefData {
            id: "d1".into(),
            hostname: "*.example.com".into(),
            is_wildcard: true,
        }];
        assert_eq!(export_stem(&ds, "c1"), "_.example.com");
        assert_eq!(export_stem(&[], "c1"), "certificate-c1");
    }

    #[test]
    fn build_pfx_roundtrip() {
        let (leaf_pem, key_pem) = self_signed_materials();
        let pfx = build_pfx("example.com", &leaf_pem, "", &key_pem, "s3cret").expect("build pfx");
        // DER SEQUENCE
        assert_eq!(pfx[0], 0x30);
        let ks = p12_keystore::KeyStore::from_pkcs12(&pfx, "s3cret").expect("parse pfx");
        let (alias, chain) = ks.private_key_chain().expect("key chain entry");
        assert_eq!(alias, "example.com");
        assert_eq!(chain.chain().len(), 1);
        // 口令错误应解不开
        assert!(p12_keystore::KeyStore::from_pkcs12(&pfx, "wrong").is_err());
    }

    #[test]
    fn build_pfx_rejects_bad_pem() {
        assert!(build_pfx("x", "not pem", "", "also not pem", "p").is_err());
    }
}
