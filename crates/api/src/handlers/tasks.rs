//! tasks handlers —— list/detail/logs 读取;retry(派生新任务 TT7)/ cancel(TT5/TT6 + 证书回退)真实。

use crate::dto::{self, Page, TaskDetail, TaskLogEntry, TaskSummary};
use crate::error::ApiResult;
use crate::parse::{parse_enum_list, parse_enum_opt};
use crate::req::{LogsQuery, TaskListQuery};
use crate::state::AppState;
use autohttps_core::enums::{TaskStatus, TaskTrigger, TaskType};
use autohttps_core::services::tasks;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

pub async fn list(
    State(st): State<AppState>,
    Query(q): Query<TaskListQuery>,
) -> ApiResult<Json<Page<TaskSummary>>> {
    let filter = tasks::TaskListFilter {
        page: q.page,
        page_size: q.page_size,
        task_type: parse_enum_opt::<TaskType>("taskType", &q.task_type)?,
        statuses: parse_enum_list::<TaskStatus>("status", &q.status)?,
        certificate_id: q.certificate_id,
        trigger: parse_enum_opt::<TaskTrigger>("trigger", &q.trigger)?,
        date_from: q.date_from,
        date_to: q.date_to,
        sort: q.sort,
        order: q.order,
    };
    let paged = tasks::list(&st.ctx, filter).await?;
    Ok(Json(dto::page_of(paged, dto::task_summary)))
}

pub async fn get(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TaskDetail>> {
    let data = tasks::get(&st.ctx, &id).await?;
    Ok(Json(dto::task_detail(data)))
}

pub async fn logs(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> ApiResult<Json<Page<TaskLogEntry>>> {
    let paged = tasks::logs(&st.ctx, &id, q.after_seq, q.page, q.page_size).await?;
    Ok(Json(dto::page_of(paged, dto::task_log_entry)))
}

/// 手动重试(B1,TT7)—— 对失败任务派生同类型新任务;原失败任务留痕。202。
pub async fn retry(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<TaskDetail>)> {
    let detail = tasks::retry_task(&st.ctx, &id).await?;
    Ok((StatusCode::ACCEPTED, Json(dto::task_detail(detail))))
}

/// 取消(B2,TT5/TT6)—— queued→200 直接取消;running→202 尽力取消;驱动证书回退 T21–T24。
pub async fn cancel(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<TaskDetail>)> {
    let outcome = tasks::cancel_task(&st.ctx, &id).await?;
    let code = if outcome.was_running {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };
    Ok((code, Json(dto::task_detail(outcome.detail))))
}
