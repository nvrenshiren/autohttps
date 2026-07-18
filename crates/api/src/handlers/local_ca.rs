//! local-ca handlers —— 根 CA list / detail / create / import 真实实现;export 下发公开证书 PEM。

use crate::dto::{self, Page, RootCaDetail, RootCaSummary};
use crate::error::ApiResult;
use crate::extract::JsonBody;
use crate::parse::parse_enum_opt;
use crate::req::{CreateRootCaRequest, ImportRootCaRequest, RootCaListQuery};
use crate::state::AppState;
use autohttps_core::enums::RootCaStatus;
use autohttps_core::services::local_ca;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

pub async fn list(
    State(st): State<AppState>,
    Query(q): Query<RootCaListQuery>,
) -> ApiResult<Json<Page<RootCaSummary>>> {
    let filter = local_ca::RootCaListFilter {
        page: q.page,
        page_size: q.page_size,
        status: parse_enum_opt::<RootCaStatus>("status", &q.status)?,
        sort: q.sort,
        order: q.order,
    };
    let paged = local_ca::list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::root_ca_summary)))
}

pub async fn get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<RootCaDetail>> {
    let row = local_ca::get(&st.ctx, &id).await?;
    Ok(Json(dto::root_ca_detail(row)))
}

/// A2 / L1:本地生成密钥对并自签 → active。同步、无过渡态 → 201 + RootCaDetail。
pub async fn create(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<CreateRootCaRequest>,
) -> ApiResult<(StatusCode, Json<RootCaDetail>)> {
    let row = local_ca::create(
        &st.ctx,
        local_ca::CreateRootCaInput {
            name: body.name,
            validity_days: body.validity_days,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(dto::root_ca_detail(row))))
}

/// A3 / L2:导入证书+配对私钥,校验后落地 → active(证书已过期则 expired)→ 201 + RootCaDetail。
pub async fn import(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<ImportRootCaRequest>,
) -> ApiResult<(StatusCode, Json<RootCaDetail>)> {
    let row = local_ca::import(
        &st.ctx,
        local_ca::ImportRootCaInput {
            name: body.name,
            cert_pem: body.cert_pem,
            private_key_pem: body.private_key_pem,
            key_passphrase: body.key_passphrase,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(dto::root_ca_detail(row))))
}

/// A4:导出根 CA **公开证书** PEM(不含私钥,LC4)。二进制下载(common §5);只读、不改状态。
pub async fn export(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    let row = local_ca::get(&st.ctx, &id).await?;
    let pem = row.root_ca.cert_pem;
    let disposition = format!("attachment; filename=\"root-ca-{id}.pem\"");
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-pem-file".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        pem,
    )
        .into_response())
}
