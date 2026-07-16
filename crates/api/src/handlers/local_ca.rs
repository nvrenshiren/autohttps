//! local-ca handlers —— 根 CA list+detail 真实读取;create/import/export 打桩 501(依赖 rcgen)。

use crate::dto::{self, Page, RootCaDetail, RootCaSummary};
use crate::error::{ApiError, ApiResult};
use crate::parse::parse_enum_opt;
use crate::req::RootCaListQuery;
use crate::state::AppState;
use autohttps_core::enums::RootCaStatus;
use autohttps_core::services::local_ca;
use autohttps_core::ErrorCode;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
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

pub async fn get(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<RootCaDetail>> {
    let row = local_ca::get(&st.ctx, &id).await?;
    Ok(Json(dto::root_ca_detail(row)))
}

fn stub(action: &str) -> ApiError {
    ApiError::new(ErrorCode::NotImplemented, format!("{action}:自签 CA(rcgen)为里程碑1 打桩"))
}

pub async fn create() -> ApiResult<StatusCode> {
    Err(stub("创建根 CA"))
}
pub async fn import() -> ApiResult<StatusCode> {
    Err(stub("导入根 CA"))
}
pub async fn export(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(stub("导出根 CA"))
}
