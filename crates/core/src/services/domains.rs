//! 域名服务(API domains)—— 全实现 CRUD。证书态为**只读投影**(经 `certificate_domains` 反查)。

use crate::domain::enums::{CertificateStatus, IssuanceMethod, ValidationMethod};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::rules::worst_projection;
use crate::persistence::entities::{
    certificate_domains, certificates, domains, http01_validation_configs,
};
use crate::services::context::CoreContext;
use crate::services::pagination::{PageParams, Paged};
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;

/// 列表行:域名 + 证书态投影(worstCertificateStatus / certificateCount)。
pub struct DomainRow {
    pub domain: domains::Model,
    pub certificate_count: u64,
    pub worst_status: Option<CertificateStatus>,
}

/// 关联证书投影(详情 certificates 清单)。
pub struct DomainCertProjection {
    pub id: String,
    pub status: CertificateStatus,
    pub issuance_method: IssuanceMethod,
    pub not_after: Option<String>,
}

pub struct DomainDetailData {
    pub row: DomainRow,
    pub certificates: Vec<DomainCertProjection>,
}

/// `certificateState` 过滤 —— 按域名"最紧急"证书态投影(`worst_projection`)判定。
/// 词表(契约 domains §3):`CertificateStatus` wire 值(可逗号多值)+ `none`(无任何关联证书),不新增枚举。
/// 设计 §3.3 的"失败"桶由前端展开为 `expired,issue_failed,renewal_failed` 多值(与 tasks 队列=`queued,running` 同约定)。
#[derive(Default)]
pub struct CertStateFilter {
    pub statuses: Vec<CertificateStatus>,
    pub include_none: bool,
}

impl CertStateFilter {
    /// 空词表 ⇒ 不过滤(返回全部)。
    fn is_active(&self) -> bool {
        !self.statuses.is_empty() || self.include_none
    }

    /// 域名 worst-projection 是否命中:None(无证书)⇒ `none`;Some(s) ⇒ s ∈ statuses。
    fn matches(&self, worst: Option<CertificateStatus>) -> bool {
        match worst {
            None => self.include_none,
            Some(s) => self.statuses.contains(&s),
        }
    }
}

#[derive(Default)]
pub struct DomainListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub group: Option<String>,
    pub hostname: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    /// 证书态投影过滤(见 [`CertStateFilter`]);空 ⇒ 不过滤。
    pub certificate_state: CertStateFilter,
}

pub struct CreateDomainInput {
    pub hostname: String,
    pub group_name: Option<String>,
    pub remark: Option<String>,
    pub validation_method: Option<ValidationMethod>,
}

/// PATCH:外层 None=不改;Some(None)=清空;Some(Some(v))=设值。`hostname_attempted` 表示请求体含 hostname。
#[derive(Default)]
pub struct UpdateDomainInput {
    pub group_name: Option<Option<String>>,
    pub remark: Option<Option<String>>,
    pub validation_method: Option<Option<ValidationMethod>>,
    pub hostname_attempted: bool,
}

/// hostname 基础格式校验(格式非法 → 400 validation_failed)。
fn validate_hostname(hostname: &str) -> CoreResult<bool> {
    let h = hostname.trim();
    if h.is_empty() || h.len() > 253 {
        return Err(CoreError::validation("hostname 不能为空且不超过 253 字符"));
    }
    let is_wildcard = h.starts_with("*.");
    let labels_part = if is_wildcard { &h[2..] } else { h };
    if labels_part.is_empty() || labels_part.contains('*') {
        return Err(CoreError::validation(
            "hostname 通配符只能作为最左标签(如 *.example.com)",
        ));
    }
    let ok = labels_part.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    }) && labels_part.contains('.');
    if !ok {
        return Err(CoreError::validation("hostname 格式非法"));
    }
    Ok(is_wildcard)
}

/// 反查某域名的关联证书(现存)投影。
async fn domain_cert_projection(
    db: &DatabaseConnection,
    domain_id: &str,
) -> CoreResult<Vec<DomainCertProjection>> {
    let links = certificate_domains::Entity::find()
        .filter(certificate_domains::Column::DomainId.eq(domain_id))
        .all(db)
        .await?;
    let cert_ids: Vec<String> = links.into_iter().map(|l| l.certificate_id).collect();
    if cert_ids.is_empty() {
        return Ok(vec![]);
    }
    let certs = certificates::Entity::find()
        .filter(certificates::Column::Id.is_in(cert_ids))
        .all(db)
        .await?;
    Ok(certs
        .into_iter()
        .map(|c| DomainCertProjection {
            id: c.id,
            status: c.status,
            issuance_method: c.issuance_method,
            not_after: c.not_after,
        })
        .collect())
}

async fn build_row(db: &DatabaseConnection, domain: domains::Model) -> CoreResult<DomainRow> {
    let projections = domain_cert_projection(db, &domain.id).await?;
    let statuses: Vec<CertificateStatus> = projections.iter().map(|p| p.status).collect();
    Ok(DomainRow {
        certificate_count: projections.len() as u64,
        worst_status: worst_projection(&statuses),
        domain,
    })
}

pub async fn list(ctx: &CoreContext, filter: DomainListFilter) -> CoreResult<Paged<DomainRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);

    let mut query = domains::Entity::find();
    if let Some(g) = filter.group.filter(|s| !s.is_empty()) {
        query = query.filter(domains::Column::GroupName.eq(g));
    }
    if let Some(h) = filter.hostname.filter(|s| !s.is_empty()) {
        query = query.filter(domains::Column::Hostname.contains(h.as_str()));
    }

    // 排序白名单(common §3.2)
    let order = matches!(filter.order.as_deref(), Some("asc")).then_some(Order::Asc);
    let (col, default_order) = match filter.sort.as_deref() {
        None | Some("hostname") => (domains::Column::Hostname, Order::Asc),
        Some("createdAt") => (domains::Column::CreatedAt, Order::Desc),
        Some("updatedAt") => (domains::Column::UpdatedAt, Order::Desc),
        Some(other) => {
            return Err(CoreError::new(
                ErrorCode::ValidationFailed,
                format!("不支持的排序字段: {other}"),
            ))
        }
    };
    query = query.order_by(col, order.unwrap_or(default_order));

    // certificateState 过滤:投影(worst_projection)是应用层派生量、非 DB 列,故取符合 group/hostname/sort
    // 的全量 → 建行(含投影)→ 按投影过滤 → 在 Rust 内分页;total 基于过滤后结果(数据量小,清晰优先)。
    if filter.certificate_state.is_active() {
        let models = query.all(db).await?;
        let mut rows = Vec::with_capacity(models.len());
        for m in models {
            let row = build_row(db, m).await?;
            if filter.certificate_state.matches(row.worst_status) {
                rows.push(row);
            }
        }
        let total = rows.len() as u64;
        let skip = (page.zero_based() * page.page_size) as usize;
        let items = rows
            .into_iter()
            .skip(skip)
            .take(page.page_size as usize)
            .collect();
        return Ok(Paged {
            items,
            page: page.page,
            page_size: page.page_size,
            total,
        });
    }

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;

    let mut items = Vec::with_capacity(models.len());
    for m in models {
        items.push(build_row(db, m).await?);
    }
    Ok(Paged {
        items,
        page: page.page,
        page_size: page.page_size,
        total,
    })
}

pub async fn get(ctx: &CoreContext, id: &str) -> CoreResult<DomainDetailData> {
    let db = &ctx.db;
    let domain = domains::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::DomainNotFound, "域名不存在"))?;
    let certificates = domain_cert_projection(db, id).await?;
    let statuses: Vec<CertificateStatus> = certificates.iter().map(|p| p.status).collect();
    let row = DomainRow {
        certificate_count: certificates.len() as u64,
        worst_status: worst_projection(&statuses),
        domain,
    };
    Ok(DomainDetailData { row, certificates })
}

pub async fn create(ctx: &CoreContext, input: CreateDomainInput) -> CoreResult<DomainDetailData> {
    let db = &ctx.db;
    let is_wildcard = validate_hostname(&input.hostname)?;
    let hostname = input.hostname.trim().to_string();

    // 通配符 ⇒ 验证方式须 dns_01(共享规则,domains §2.3)
    if is_wildcard && matches!(input.validation_method, Some(ValidationMethod::Http01)) {
        return Err(CoreError::new(
            ErrorCode::WildcardRequiresDns01,
            "通配符域名的验证方式必须为 dns_01",
        ));
    }

    // 同实例唯一(B1)
    let exists = domains::Entity::find()
        .filter(domains::Column::Hostname.eq(&hostname))
        .one(db)
        .await?;
    if exists.is_some() {
        return Err(CoreError::new(
            ErrorCode::DomainAlreadyExists,
            "该 hostname 已存在",
        ));
    }

    let now = now_rfc3339();
    let model = domains::ActiveModel {
        id: Set(new_id()),
        hostname: Set(hostname),
        is_wildcard: Set(is_wildcard),
        validation_method: Set(input.validation_method),
        group_name: Set(input.group_name.filter(|s| !s.is_empty())),
        remark: Set(input.remark.filter(|s| !s.is_empty())),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    };
    let domain = model.insert(db).await?;
    Ok(DomainDetailData {
        row: DomainRow {
            domain,
            certificate_count: 0,
            worst_status: None,
        },
        certificates: vec![],
    })
}

pub async fn update(
    ctx: &CoreContext,
    id: &str,
    input: UpdateDomainInput,
) -> CoreResult<DomainDetailData> {
    let db = &ctx.db;
    if input.hostname_attempted {
        return Err(CoreError::new(
            ErrorCode::HostnameImmutable,
            "hostname 不可修改(改名 = 删除后重新新增)",
        ));
    }
    let domain = domains::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::DomainNotFound, "域名不存在"))?;

    // 通配符约束(共享规则)
    if domain.is_wildcard {
        if let Some(Some(ValidationMethod::Http01)) = input.validation_method {
            return Err(CoreError::new(
                ErrorCode::WildcardRequiresDns01,
                "通配符域名的验证方式必须为 dns_01",
            ));
        }
    }

    let mut active: domains::ActiveModel = domain.into();
    if let Some(g) = input.group_name {
        active.group_name = Set(g.filter(|s| !s.is_empty()));
    }
    if let Some(r) = input.remark {
        active.remark = Set(r.filter(|s| !s.is_empty()));
    }
    if let Some(vm) = input.validation_method {
        active.validation_method = Set(vm);
    }
    active.updated_at = Set(now_rfc3339());
    active.update(db).await?;

    get(ctx, id).await
}

pub async fn delete(ctx: &CoreContext, id: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let domain = domains::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::DomainNotFound, "域名不存在"))?;

    // 前置硬拦截:被任一现存证书关联(DECD3)
    let cert_count = certificate_domains::Entity::find()
        .filter(certificate_domains::Column::DomainId.eq(&domain.id))
        .count(db)
        .await?;
    if cert_count > 0 {
        return Err(CoreError::new(
            ErrorCode::DomainHasCertificates,
            "该域名仍被证书关联,不可删除",
        )
        .with_details(serde_json::json!({ "certificateCount": cert_count })));
    }

    // 关联 HTTP-01 配置随域名删除(FK CASCADE 兜底,显式清理更稳)
    http01_validation_configs::Entity::delete_by_id(&domain.id)
        .exec(db)
        .await?;
    domains::Entity::delete_by_id(&domain.id).exec(db).await?;
    Ok(())
}
