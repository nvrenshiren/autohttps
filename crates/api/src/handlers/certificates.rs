//! certificates handlers —— list/detail/create/delete/revoke/renew/retry/export 全真实实现。

use crate::dto::{self, CertificateDetail, CertificateSummary, Page};
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::parse::{parse_enum_list, parse_enum_opt};
use crate::req::{CertListQuery, ExportQuery, IssueCertificateRequest};
use crate::state::AppState;
use autohttps_core::enums::{CertificateStatus, IssuanceMethod};
use autohttps_core::services::certificates::{self, ExportPart};
use autohttps_core::ErrorCode;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
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

/// 续签 / 再获取(C1,T7/T9/T17/T20 → renewing)→ 入队 renew 任务;执行器承接 self_signed 重签。202。
pub async fn renew(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<CertificateDetail>)> {
    let data = certificates::renew(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::cert_detail(data))))
}

/// 失败重试(B2·C3,issue_failed→T5 / renewal_failed→T14)→ 派生新任务(TT7)。202。
pub async fn retry(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<CertificateDetail>)> {
    let data = certificates::retry(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::cert_detail(data))))
}

/// 吊销(D1,T8/T11/T16 → revoking)→ 入队 revoke 任务;执行器承接 self_signed 作废 → revoked。202。
pub async fn revoke(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<CertificateDetail>)> {
    let data = certificates::revoke(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::cert_detail(data))))
}

/// 导出叶子/链/私钥(E1,§2.8)—— 二进制 PEM 下载(`application/x-pem-file`);跨状态只读动作。
/// `parts` 非法值 → 400 validation_failed;含私钥未确认 → 422;无文件态 → 409(见服务层)。
pub async fn export(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ExportQuery>,
) -> ApiResult<Response> {
    // format:MVP 仅 pem
    if let Some(f) = q.format.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if !f.eq_ignore_ascii_case("pem") {
            return Err(ApiError::new(ErrorCode::ValidationFailed, format!("不支持的导出格式: {f}")));
        }
    }
    let parts = parse_export_parts(q.parts.as_deref())?;
    let input = certificates::ExportInput {
        parts,
        acknowledge_key_export: q.acknowledge_key_export.unwrap_or(false),
    };
    let bundle = certificates::export(&st.ctx, &id, input).await?;
    let disposition = format!("attachment; filename=\"{}\"", bundle.filename);
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-pem-file".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bundle.pem,
    )
        .into_response())
}

/// 解析 `parts=<逗号分隔:leaf|chain|fullchain|private_key>`(默认 fullchain);非法值 → 400。
fn parse_export_parts(raw: Option<&str>) -> ApiResult<Vec<ExportPart>> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty()).unwrap_or("fullchain");
    let mut parts = Vec::new();
    for token in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let p = ExportPart::parse(token).ok_or_else(|| {
            ApiError::new(ErrorCode::ValidationFailed, format!("非法的导出内容: {token}"))
        })?;
        parts.push(p);
    }
    if parts.is_empty() {
        return Err(ApiError::new(ErrorCode::ValidationFailed, "parts 不能为空"));
    }
    Ok(parts)
}
