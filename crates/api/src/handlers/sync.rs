//! sync handlers —— WebDAV 备份同步(配置读写 / 测试连接 / 备份 / 列远端 / 恢复)。

use crate::error::ApiResult;
use crate::extract::JsonBody;
use crate::req::{BackupNowRequest, PutSyncConfigRequest, RestoreRequest};
use crate::state::AppState;
use autohttps_core::services::sync;
use axum::extract::State;
use axum::Json;

pub async fn get_config(State(st): State<AppState>) -> ApiResult<Json<sync::SyncConfigView>> {
    Ok(Json(sync::get_config(&st.ctx).await?))
}

pub async fn put_config(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<PutSyncConfigRequest>,
) -> ApiResult<Json<sync::SyncConfigView>> {
    let input = sync::SaveSyncConfigInput {
        server_url: body.server_url,
        remote_dir: body.remote_dir,
        username: body.username,
        password: body.password,
    };
    Ok(Json(sync::save_config(&st.ctx, input).await?))
}

pub async fn delete_config(State(st): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    sync::delete_config(&st.ctx).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn test(State(st): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    sync::test_connection(&st.ctx).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn backup(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<BackupNowRequest>,
) -> ApiResult<Json<sync::RemoteBackupItem>> {
    Ok(Json(sync::backup_now(&st.ctx, &body.passphrase).await?))
}

pub async fn list_backups(
    State(st): State<AppState>,
) -> ApiResult<Json<Vec<sync::RemoteBackupItem>>> {
    Ok(Json(sync::list_backups(&st.ctx).await?))
}

pub async fn restore(
    State(st): State<AppState>,
    JsonBody(body): JsonBody<RestoreRequest>,
) -> ApiResult<Json<sync::RestoreOutcome>> {
    Ok(Json(
        sync::restore(&st.ctx, &body.remote_name, &body.passphrase).await?,
    ))
}
