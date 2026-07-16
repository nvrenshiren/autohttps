//! acme handlers —— 账户/挑战 list+detail、http01 get/put 真实读取;在线交互动作打桩 501。

use crate::dto::{
    self, AcmeAccountDetail, AcmeAccountSummary, ChallengeDetail, ChallengeSummary, Http01Config, Page,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::parse::parse_enum_opt;
use crate::req::{AccountListQuery, ChallengeListQuery, PutHttp01ConfigRequest};
use crate::state::AppState;
use autohttps_core::enums::{AcmeAccountStatus, ChallengeStatus};
use autohttps_core::services::acme;
use autohttps_core::ErrorCode;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

pub async fn accounts_list(
    State(st): State<AppState>,
    Query(q): Query<AccountListQuery>,
) -> ApiResult<Json<Page<AcmeAccountSummary>>> {
    let filter = acme::AccountListFilter {
        page: q.page,
        page_size: q.page_size,
        status: parse_enum_opt::<AcmeAccountStatus>("status", &q.status)?,
    };
    let paged = acme::accounts_list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::acme_account_summary)))
}

pub async fn account_get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<AcmeAccountDetail>> {
    let row = acme::account_get(&st.ctx, &id).await?;
    Ok(Json(dto::acme_account_detail(row)))
}

pub async fn challenges_list(
    State(st): State<AppState>,
    Query(q): Query<ChallengeListQuery>,
) -> ApiResult<Json<Page<ChallengeSummary>>> {
    let filter = acme::ChallengeListFilter {
        page: q.page,
        page_size: q.page_size,
        task_id: q.task_id,
        domain_id: q.domain_id,
        status: parse_enum_opt::<ChallengeStatus>("status", &q.status)?,
        certificate_id: q.certificate_id,
    };
    let paged = acme::challenges_list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::challenge_summary)))
}

pub async fn challenge_get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ChallengeDetail>> {
    let row = acme::challenge_get(&st.ctx, &id).await?;
    Ok(Json(dto::challenge_detail(row)))
}

pub async fn http01_get(
    State(st): State<AppState>,
    Path(domain_id): Path<String>,
) -> ApiResult<Json<Http01Config>> {
    let m = acme::http01_get(&st.ctx, &domain_id).await?;
    Ok(Json(dto::http01_config(m)))
}

pub async fn http01_put(
    State(st): State<AppState>,
    Path(domain_id): Path<String>,
    JsonBody(body): JsonBody<PutHttp01ConfigRequest>,
) -> ApiResult<Json<Http01Config>> {
    let m = acme::http01_put(&st.ctx, &domain_id, body.webroot_path).await?;
    Ok(Json(dto::http01_config(m)))
}

// --- 在线交互动作:里程碑1 打桩(依赖 instant-acme)---

fn stub(action: &str) -> ApiError {
    ApiError::new(ErrorCode::NotImplemented, format!("{action}:ACME 在线交互为里程碑1 打桩"))
}

pub async fn account_create() -> ApiResult<StatusCode> {
    Err(stub("配置并注册账户"))
}
pub async fn account_patch(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("编辑账户"))
}
pub async fn account_retry(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("账户注册重试"))
}
pub async fn account_delete(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("移除账户"))
}
pub async fn challenge_confirm(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("确认 DNS-01 TXT"))
}
pub async fn challenge_retry(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("挑战重试"))
}
pub async fn dns_precheck(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("DNS-01 预检"))
}
