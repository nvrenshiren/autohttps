//! settings handlers —— 单例读取 / 修改(API settings)。

use crate::dto::{self, SettingsView};
use crate::error::ApiResult;
use crate::extract::JsonBody;
use crate::req::UpdateSettingsRequest;
use crate::state::AppState;
use autohttps_core::services::settings;
use axum::extract::State;
use axum::Json;

pub async fn get(State(st): State<AppState>) -> ApiResult<Json<SettingsView>> {
    let model = settings::get_or_init(&st.ctx).await?;
    Ok(Json(dto::settings_view(model, st.ctx.run_mode)))
}

pub async fn patch(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<UpdateSettingsRequest>,
) -> ApiResult<Json<SettingsView>> {
    let input = settings::UpdateSettingsInput {
        renewal_advance_days: body.renewal_advance_days,
        auto_renew_enabled: body.auto_renew_enabled,
        default_acme_account_id: body.default_acme_account_id,
        autostart_enabled: body.autostart_enabled,
        listen_address: body.listen_address,
        listen_port: body.listen_port,
        data_storage_path_attempted: body.data_storage_path.is_some(),
    };
    let model = settings::update(&st.ctx, input).await?;
    Ok(Json(dto::settings_view(model, st.ctx.run_mode)))
}
