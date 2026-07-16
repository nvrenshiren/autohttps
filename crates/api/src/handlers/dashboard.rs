//! dashboard handler —— 聚合总览(纯只读,API dashboard)。

use crate::dto::{self, DashboardOverview};
use crate::error::ApiResult;
use crate::state::AppState;
use autohttps_core::services::dashboard;
use axum::extract::State;
use axum::Json;

pub async fn overview(State(st): State<AppState>) -> ApiResult<Json<DashboardOverview>> {
    let data = dashboard::overview(&st.ctx).await?;
    Ok(Json(dto::dashboard_overview(data)))
}
