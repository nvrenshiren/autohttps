//! acme handlers —— 账户/挑战 list+detail、http01 get/put 真实读取;在线交互动作打桩 501。

use crate::dto::{
    self, AcmeAccountDetail, AcmeAccountSummary, ChallengeDetail, ChallengeSummary, Http01Config, Page,
};
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::parse::parse_enum_opt;
use crate::req::{
    AccountListQuery, ChallengeListQuery, PatchAcmeAccountRequest, PutHttp01ConfigRequest,
    RegisterAcmeAccountRequest,
};
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

/// 配置并注册账户(A1,AT1)。校验通过 → 插 `registering` + 后台异步向 CA 注册 → **202** + 详情;
/// 终态经 SSE `acme_account_status_changed` 回推(acme api §2.1)。
pub async fn account_create(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<RegisterAcmeAccountRequest>,
) -> ApiResult<(StatusCode, Json<AcmeAccountDetail>)> {
    let row = acme::create_account(
        &st.ctx,
        acme::RegisterAccountInput {
            directory_url: body.directory_url,
            ca_label: body.ca_label,
            contact_email: body.contact_email,
            tos_agreed: body.tos_agreed,
        },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(dto::acme_account_detail(row))))
}

/// 编辑联系邮箱(A3,跨状态动作)。仅 `registered` 可编辑,否则 409 account_state_invalid。→ 200。
pub async fn account_patch(
    State(st): State<AppState>,
    Path(id): Path<String>,
    JsonBody(body): JsonBody<PatchAcmeAccountRequest>,
) -> ApiResult<Json<AcmeAccountDetail>> {
    let row = acme::patch_account(&st.ctx, &id, body.contact_email).await?;
    Ok(Json(dto::acme_account_detail(row)))
}

/// 注册失败重试(A4,AT4)。仅 `registration_failed` → `registering`;终态经 SSE 回推。→ **202**。
pub async fn account_retry(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<AcmeAccountDetail>)> {
    let row = acme::retry_account(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::acme_account_detail(row))))
}

/// 移除账户(A5,AT5)。证书/settings 引用置空(SET NULL)、清账户密钥。→ **204**。
pub async fn account_delete(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    acme::delete_account(&st.ctx, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DNS-01 确认已添加 TXT(B4,CT4)→ 通知 CA 校验、全部就绪续推 finalize;终态经 SSE。→ **202** + 挑战详情。
pub async fn challenge_confirm(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<ChallengeDetail>)> {
    let row = acme::confirm_challenge(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::challenge_detail(row))))
}

/// 挑战失败重试(B5,CT7)→ 重建订单取新挑战(委派证书重试,派生新任务)。→ **202**。
pub async fn challenge_retry(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    acme::retry_challenge(&st.ctx, &id).await?;
    Ok(StatusCode::ACCEPTED)
}

// --- 仍打桩:DNS-01 提交前本地预检(B4 可选,需 hickory-resolver;确认流不依赖之)---
pub async fn dns_precheck(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(ApiError::new(
        ErrorCode::NotImplemented,
        "DNS-01 本地预检(dns-precheck)留后续:需 hickory-resolver;确认流不依赖之",
    ))
}
