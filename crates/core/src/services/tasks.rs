//! 任务服务(API tasks)—— list / detail / logs 读取;retry(派生新任务 TT7)/ cancel(TT5/TT6 +
//! 驱动证书回退 T21–T24)为真实实现。取消/重试的证书联动收敛于 certificates 服务(证书状态机唯一真相)。

use crate::domain::enums::{TaskStatus, TaskTrigger, TaskType};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    certificate_domains, certificates, domains, task_log_entries, tasks,
};
use crate::services::certificates as cert_svc;
use crate::services::context::CoreContext;
use crate::services::dashboard;
use crate::services::pagination::{PageParams, Paged};
use crate::util::now_rfc3339;
use sea_orm::*;

pub struct TaskRow {
    pub task: tasks::Model,
    pub certificate_deleted: bool,
    /// 关联证书 hostname(证书存在时);已删除为 None。
    pub certificate_domains: Option<Vec<String>>,
}

pub struct TaskCertRefData {
    pub id: String,
    pub status: crate::domain::enums::CertificateStatus,
    pub domains: Vec<String>,
}

pub struct TaskDetailData {
    pub row: TaskRow,
    pub parent_task_id: Option<String>,
    pub child_task_ids: Vec<String>,
    pub certificate: Option<TaskCertRefData>,
}

#[derive(Default)]
pub struct TaskListFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub task_type: Option<TaskType>,
    pub statuses: Vec<TaskStatus>,
    pub certificate_id: Option<String>,
    pub trigger: Option<TaskTrigger>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

async fn cert_hostnames(db: &DatabaseConnection, cert_id: &str) -> CoreResult<Vec<String>> {
    let ids: Vec<String> = certificate_domains::Entity::find()
        .filter(certificate_domains::Column::CertificateId.eq(cert_id))
        .all(db)
        .await?
        .into_iter()
        .map(|l| l.domain_id)
        .collect();
    if ids.is_empty() {
        return Ok(vec![]);
    }
    Ok(domains::Entity::find()
        .filter(domains::Column::Id.is_in(ids))
        .all(db)
        .await?
        .into_iter()
        .map(|d| d.hostname)
        .collect())
}

async fn build_row(db: &DatabaseConnection, task: tasks::Model) -> CoreResult<TaskRow> {
    let cert = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?;
    let (certificate_deleted, certificate_domains) = match cert {
        Some(_) => (false, Some(cert_hostnames(db, &task.certificate_id).await?)),
        None => (true, None),
    };
    Ok(TaskRow {
        task,
        certificate_deleted,
        certificate_domains,
    })
}

pub async fn list(ctx: &CoreContext, filter: TaskListFilter) -> CoreResult<Paged<TaskRow>> {
    let db = &ctx.db;
    let page = PageParams::normalize(filter.page, filter.page_size);

    let mut query = tasks::Entity::find();
    if let Some(t) = filter.task_type {
        query = query.filter(tasks::Column::TaskType.eq(t));
    }
    if !filter.statuses.is_empty() {
        query = query.filter(tasks::Column::Status.is_in(filter.statuses.clone()));
    }
    if let Some(c) = filter.certificate_id.filter(|s| !s.is_empty()) {
        query = query.filter(tasks::Column::CertificateId.eq(c));
    }
    if let Some(tr) = filter.trigger {
        query = query.filter(tasks::Column::Trigger.eq(tr));
    }
    if let Some(from) = filter.date_from.filter(|s| !s.is_empty()) {
        query = query.filter(tasks::Column::QueuedAt.gte(from));
    }
    if let Some(to) = filter.date_to.filter(|s| !s.is_empty()) {
        query = query.filter(tasks::Column::QueuedAt.lte(to));
    }

    let order = matches!(filter.order.as_deref(), Some("asc")).then_some(Order::Asc);
    let (col, default_order) = match filter.sort.as_deref() {
        None | Some("queuedAt") => (tasks::Column::QueuedAt, Order::Desc),
        Some("finishedAt") => (tasks::Column::FinishedAt, Order::Desc),
        Some(other) => {
            return Err(CoreError::new(
                ErrorCode::ValidationFailed,
                format!("不支持的排序字段: {other}"),
            ))
        }
    };
    query = query.order_by(col, order.unwrap_or(default_order));

    let paginator = query.paginate(db, page.page_size);
    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page.zero_based()).await?;
    let mut items = Vec::with_capacity(models.len());
    for t in models {
        items.push(build_row(db, t).await?);
    }
    Ok(Paged {
        items,
        page: page.page,
        page_size: page.page_size,
        total,
    })
}

pub async fn get(ctx: &CoreContext, id: &str) -> CoreResult<TaskDetailData> {
    let db = &ctx.db;
    let task = tasks::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::TaskNotFound, "任务不存在"))?;

    let parent_task_id = task.parent_task_id.clone();
    let child_task_ids: Vec<String> = tasks::Entity::find()
        .filter(tasks::Column::ParentTaskId.eq(id))
        .all(db)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect();

    let cert_model = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?;
    let certificate = match cert_model {
        Some(c) => Some(TaskCertRefData {
            id: c.id.clone(),
            status: c.status,
            domains: cert_hostnames(db, &c.id).await?,
        }),
        None => None,
    };

    let row = build_row(db, task).await?;
    Ok(TaskDetailData {
        row,
        parent_task_id,
        child_task_ids,
        certificate,
    })
}

pub async fn logs(
    ctx: &CoreContext,
    id: &str,
    after_seq: Option<i32>,
    page: Option<u64>,
    page_size: Option<u64>,
) -> CoreResult<Paged<task_log_entries::Model>> {
    let db = &ctx.db;
    // 任务须存在
    if tasks::Entity::find_by_id(id).one(db).await?.is_none() {
        return Err(CoreError::new(ErrorCode::TaskNotFound, "任务不存在"));
    }
    let params = PageParams::normalize(page, page_size);
    let mut query =
        task_log_entries::Entity::find().filter(task_log_entries::Column::TaskId.eq(id));
    if let Some(seq) = after_seq {
        query = query.filter(task_log_entries::Column::Seq.gt(seq));
    }
    query = query.order_by_asc(task_log_entries::Column::Seq);

    let paginator = query.paginate(db, params.page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(params.zero_based()).await?;
    Ok(Paged {
        items,
        page: params.page,
        page_size: params.page_size,
        total,
    })
}

/// 取消结果:`was_running` 供 handler 决定 202(running,尽力而为)/ 200(queued)。
pub struct CancelOutcome {
    pub was_running: bool,
    pub detail: TaskDetailData,
}

/// 取消任务(B2,TT5/TT6)。queued→已取消(TT5);running→尽力取消(TT6,在途 CA 操作可能仍生效,
/// 由证书下次扫描据实校正 DT2)。随后驱动证书回退 T21–T24(certificates 状态机唯一真相)。
pub async fn cancel_task(ctx: &CoreContext, id: &str) -> CoreResult<CancelOutcome> {
    let db = &ctx.db;
    let task = tasks::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::TaskNotFound, "任务不存在"))?;

    if !matches!(task.status, TaskStatus::Queued | TaskStatus::Running) {
        return Err(
            CoreError::new(ErrorCode::TaskNotCancellable, "终态任务不可取消")
                .with_details(serde_json::json!({ "currentStatus": task.status })),
        );
    }
    let was_running = matches!(task.status, TaskStatus::Running);

    let now = now_rfc3339();
    let mut a: tasks::ActiveModel = task.clone().into();
    a.status = Set(TaskStatus::Cancelled);
    a.finished_at = Set(Some(now.clone()));
    a.result_summary = Set(Some("任务已取消".to_string()));
    a.updated_at = Set(now);
    let task = a.update(db).await?;
    ctx.emit(DomainEvent::TaskStatusChanged {
        task_id: task.id.clone(),
        certificate_id: task.certificate_id.clone(),
        status: TaskStatus::Cancelled,
    });

    // 驱动证书回退(T21–T24);证书状态机唯一真相,本模块只触发(其内部发 certificate_status_changed)。
    cert_svc::rollback_on_cancel(ctx, &task).await?;
    dashboard::emit_changed(ctx).await;

    let detail = get(ctx, id).await?;
    Ok(CancelOutcome {
        was_running,
        detail,
    })
}

/// 手动重试(B1,TT7)。仅 `failed` 可重试 → 派生同类型、同证书新任务(trigger=manual、attempt+1、
/// parent=原任务);原失败任务留于 `failed`。关联证书已删除 → `certificate_deleted`。返回原任务详情
/// (childTaskIds 已含新派生任务)。
pub async fn retry_task(ctx: &CoreContext, id: &str) -> CoreResult<TaskDetailData> {
    let db = &ctx.db;
    let task = tasks::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::TaskNotFound, "任务不存在"))?;

    if task.status != TaskStatus::Failed {
        return Err(
            CoreError::new(ErrorCode::TaskNotRetryable, "仅失败任务可重试")
                .with_details(serde_json::json!({ "currentStatus": task.status })),
        );
    }
    // 重试前校验证书仍存在(DB §2.3,避免对已删证书误触发)
    let cert = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertificateDeleted, "关联证书已删除,不可重试"))?;

    // 派生 + 驱动证书回进行中态(收敛于 certificates 服务;其内部发 certificate/task 事件)
    cert_svc::derive_retry_from_task(ctx, &cert, &task).await?;
    dashboard::emit_changed(ctx).await;

    get(ctx, id).await
}
