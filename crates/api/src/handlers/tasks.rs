//! tasks handlers —— list/detail/logs 真实读取;retry/cancel 打桩 501(依赖执行器 + 证书回退联动)。

use crate::dto::{self, Page, TaskDetail, TaskLogEntry, TaskSummary};
use crate::error::{ApiError, ApiResult};
use crate::parse::{parse_enum_list, parse_enum_opt};
use crate::req::{LogsQuery, TaskListQuery};
use crate::state::AppState;
use autohttps_core::enums::{TaskStatus, TaskTrigger, TaskType};
use autohttps_core::services::tasks;
use autohttps_core::ErrorCode;
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

pub async fn get(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<TaskDetail>> {
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

pub async fn retry(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(ApiError::new(ErrorCode::NotImplemented, "任务重试:执行器为里程碑1 打桩"))
}

pub async fn cancel(Path(_id): Path<String>) -> ApiResult<StatusCode> {
    Err(ApiError::new(ErrorCode::NotImplemented, "任务取消:执行器 + 证书回退联动为里程碑1 打桩"))
}
