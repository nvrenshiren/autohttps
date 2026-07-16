//! 任务服务(API tasks)—— list / detail / logs 为真实读取。
//! retry / cancel 依赖执行器 + 证书回退联动,在 api 层打桩 501。

use crate::domain::enums::{TaskStatus, TaskTrigger, TaskType};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{certificate_domains, certificates, domains, task_log_entries, tasks};
use crate::services::context::CoreContext;
use crate::services::pagination::{Paged, PageParams};
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
    let cert = certificates::Entity::find_by_id(&task.certificate_id).one(db).await?;
    let (certificate_deleted, certificate_domains) = match cert {
        Some(_) => (false, Some(cert_hostnames(db, &task.certificate_id).await?)),
        None => (true, None),
    };
    Ok(TaskRow { task, certificate_deleted, certificate_domains })
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
    Ok(Paged { items, page: page.page, page_size: page.page_size, total })
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

    let cert_model = certificates::Entity::find_by_id(&task.certificate_id).one(db).await?;
    let certificate = match cert_model {
        Some(c) => Some(TaskCertRefData {
            id: c.id.clone(),
            status: c.status,
            domains: cert_hostnames(db, &c.id).await?,
        }),
        None => None,
    };

    let row = build_row(db, task).await?;
    Ok(TaskDetailData { row, parent_task_id, child_task_ids, certificate })
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
    let mut query = task_log_entries::Entity::find()
        .filter(task_log_entries::Column::TaskId.eq(id));
    if let Some(seq) = after_seq {
        query = query.filter(task_log_entries::Column::Seq.gt(seq));
    }
    query = query.order_by_asc(task_log_entries::Column::Seq);

    let paginator = query.paginate(db, params.page_size);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(params.zero_based()).await?;
    Ok(Paged { items, page: params.page, page_size: params.page_size, total })
}
