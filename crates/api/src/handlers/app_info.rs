//! app-info handler —— 运行形态 + 版本(common §6.2)。前端据 runMode 做仅桌面/仅服务器显隐。

use crate::dto::AppInfo;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;

pub async fn get(State(st): State<AppState>) -> ApiResult<Json<AppInfo>> {
    Ok(Json(AppInfo {
        run_mode: st.ctx.run_mode,
        app_version: st.ctx.app_version.clone(),
    }))
}
