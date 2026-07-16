//! certificates handlers —— list/detail/create/delete 真实实现;renew/retry/revoke/export 打桩 501。

use crate::dto::{self, CertificateDetail, CertificateSummary, Page};
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::parse::{parse_enum_list, parse_enum_opt};
use crate::req::{CertListQuery, IssueCertificateRequest};
use crate::state::AppState;
use autohttps_core::enums::{CertificateStatus, IssuanceMethod};
use autohttps_core::services::certificates;
use autohttps_core::ErrorCode;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

pub async fn list(
    State(st): State<AppState>,
    Query(q): Query<CertListQuery>,
) -> ApiResult<Json<Page<CertificateSummary>>> {
    let statuses = parse_enum_list::<CertificateStatus>("status", &q.status)?;
    let issuance_method = parse_enum_opt::<IssuanceMethod>("issuanceMethod", &q.issuance_method)?;
    let filter = certificates::CertListFilter {
        page: q.page,
        page_size: q.page_size,
        statuses,
        issuance_method,
        domain: q.domain,
        sort: q.sort,
        order: q.order,
    };
    let paged = certificates::list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::cert_summary)))
}

pub async fn get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<CertificateDetail>> {
    let data = certificates::get(&st.ctx, &id).await?;
    Ok(Json(dto::cert_detail(data)))
}

pub async fn create(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<IssueCertificateRequest>,
) -> ApiResult<(StatusCode, Json<CertificateDetail>)> {
    let input = certificates::IssueCertInput {
        issuance_method: body.issuance_method,
        domain_ids: body.domain_ids,
        acme_account_id: body.acme_account_id,
        root_ca_id: body.root_ca_id,
    };
    // 创建 pending_issue 条目 + 入队 issue 任务;终态经 SSE 回推(执行器打桩)。202 已受理。
    let data = certificates::create(&st.ctx, input).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::cert_detail(data))))
}

pub async fn delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    certificates::delete(&st.ctx, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- 以下动作依赖任务执行器 + ACME/CA,里程碑1 打桩(TODO 实现期)---

fn stub(action: &str) -> ApiError {
    ApiError::new(
        ErrorCode::NotImplemented,
        format!("{action}:签发/续签/吊销执行器为里程碑1 打桩,尚未接入 ACME/CA"),
    )
}

pub async fn renew(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("续签"))
}

pub async fn retry(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("重试"))
}

pub async fn revoke(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("吊销"))
}

pub async fn export(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("导出"))
}
