//! domains handlers —— 全实现 CRUD(API domains)。

use crate::dto::{self, DomainDetail, DomainSummary, Page};
use crate::error::ApiResult;
use crate::extract::JsonBody;
use crate::parse::parse_enum;
use crate::req::{CreateDomainRequest, DomainListQuery, UpdateDomainRequest};
use crate::state::AppState;
use autohttps_core::enums::CertificateStatus;
use autohttps_core::services::domains;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

/// `certificateState` 词表(契约 §3):`CertificateStatus` wire 值(逗号多值)+ `none`(无证书)。
/// 投影"失败"桶 = `expired,issue_failed,renewal_failed`(前端展开)。非法值 → 400 validation_failed。
fn parse_cert_state(raw: &Option<String>) -> ApiResult<domains::CertStateFilter> {
    let mut f = domains::CertStateFilter::default();
    if let Some(raw) = raw.as_deref() {
        for tok in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if tok == "none" {
                f.include_none = true;
            } else {
                f.statuses.push(parse_enum::<CertificateStatus>("certificateState", tok)?);
            }
        }
    }
    Ok(f)
}

pub async fn list(
    State(st): State<AppState>,
    Query(q): Query<DomainListQuery>,
) -> ApiResult<Json<Page<DomainSummary>>> {
    let filter = domains::DomainListFilter {
        page: q.page,
        page_size: q.page_size,
        group: q.group,
        hostname: q.hostname,
        sort: q.sort,
        order: q.order,
        certificate_state: parse_cert_state(&q.certificate_state)?,
    };
    let paged = domains::list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::domain_summary)))
}

pub async fn get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DomainDetail>> {
    let data = domains::get(&st.ctx, &id).await?;
    Ok(Json(dto::domain_detail(data)))
}

pub async fn create(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<CreateDomainRequest>,
) -> ApiResult<(StatusCode, Json<DomainDetail>)> {
    let input = domains::CreateDomainInput {
        hostname: body.hostname,
        group_name: body.group_name,
        remark: body.remark,
        validation_method: body.validation_method,
    };
    let data = domains::create(&st.ctx, input).await?;
    Ok((StatusCode::CREATED, Json(dto::domain_detail(data))))
}

pub async fn update(
    State(st): State<AppState>,
    Path(id): Path<String>,
    JsonBody(body): JsonBody<UpdateDomainRequest>,
) -> ApiResult<Json<DomainDetail>> {
    let input = domains::UpdateDomainInput {
        group_name: body.group_name,
        remark: body.remark,
        validation_method: body.validation_method,
        hostname_attempted: body.hostname.is_some(),
    };
    let data = domains::update(&st.ctx, &id, input).await?;
    Ok(Json(dto::domain_detail(data)))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    domains::delete(&st.ctx, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
