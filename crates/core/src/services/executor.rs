//! 任务执行器(tokio worker)—— 持久队列消费者(AR5 / 决策7 / ARCHITECTURE §6.1)。
//!
//! `tasks.status=queued` 行即待办队列;单进程单 worker 按 `queued_at` FIFO 取出,
//! `queued→running→succeeded/failed`(TT2–TT4),据结果驱动证书状态机(唯一真相在 core,不复述)。
//!
//! self_signed 与 ACME 均支持签发、续签和吊销。DNS-01 挑战进入 `awaiting_manual` 后任务让出
//! worker,由 `/acme/challenges/{id}/confirm` 继续执行。日志不包含密钥材料(AR4/L6)。

use crate::domain::enums::{CertificateStatus, IssuanceMethod, TaskStatus, TaskType};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    certificate_domains, certificates, domains, task_log_entries, tasks,
};
use crate::services::context::CoreContext;
use crate::services::dashboard;
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;
use std::time::Duration;

mod acme;
mod challenges;
mod self_signed;

pub(crate) use challenges::{confirm_challenge, retry_challenge};

/// 单个可执行任务的执行结果:完成(→ 成功任务)或挂起等待手动(DNS-01,任务留 `running`)。
pub(super) enum ExecOutcome {
    /// 执行完成,携带任务结果摘要 → `running → succeeded`(TT3)。
    Done(String),
    /// DNS-01 挑战进入 `awaiting_manual`,任务让出 worker、保持 `running`,等 confirm 续推(不卡死)。
    Suspended,
}

/// 内网叶子证书默认有效期(天)。浏览器对服务器证书有效期上限约 398 天,取 365 稳妥。
pub(super) const LEAF_VALIDITY_DAYS: i64 = 365;
/// 空闲轮询间隔(无可执行任务时休眠)。
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// 单次 tick 扫描的 queued 批量上限(挑第一个本切片可执行者)。
const SCAN_BATCH: u64 = 50;

/// 启动后台执行器循环(server/desktop boot 之后调用)。返回 JoinHandle(通常无需 join,自行常驻)。
pub fn spawn(ctx: CoreContext) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("任务执行器已启动(self_signed + acme issue/renew/revoke,DNS-01 手动挂起)");
        loop {
            match tick(&ctx).await {
                // 处理了一个 → 立即继续排空,不休眠
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(POLL_INTERVAL).await,
                Err(e) => {
                    tracing::error!(error = %e, "执行器 tick 失败,稍后重试");
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }
        }
    })
}

/// 处理一个可执行任务;返回是否处理了任务(`false`=当前无 `queued` 任务,应休眠)。
///
/// 可执行 = 关联证书存在(`self_signed` / `acme` 的 issue/renew/revoke 均承接)。DNS-01 挂起的任务
/// 停在 `running`(非 `queued`),不会被本扫描重复取出,confirm 续推(见 [`confirm_challenge`])。
pub async fn tick(ctx: &CoreContext) -> CoreResult<bool> {
    let db = &ctx.db;
    let queued = tasks::Entity::find()
        .filter(tasks::Column::Status.eq(TaskStatus::Queued))
        .order_by_asc(tasks::Column::QueuedAt)
        .limit(SCAN_BATCH)
        .all(db)
        .await?;

    // 每次处理队首一个 queued 任务;外层循环反复 tick 排空(见 spawn)。
    // 挂起的 DNS-01 任务转 running、不再被本查询取出,故不会阻塞队列。
    let Some(task) = queued.into_iter().next() else {
        return Ok(false); // 当前无 queued 任务 → 休眠
    };
    let cert = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?;
    let Some(cert) = cert else {
        // 证书已删除(删除会清理未完成任务;此为兜底)→ 置失败,不再纠缠
        fail_task(ctx, &task, "关联证书已删除,任务无法执行").await?;
        return Ok(true);
    };
    claim_and_run(ctx, task, cert).await?;
    Ok(true)
}

/// 认领任务(TT2 queued→running)并执行,据结果推进任务与证书状态机。
async fn claim_and_run(
    ctx: &CoreContext,
    task: tasks::Model,
    cert: certificates::Model,
) -> CoreResult<()> {
    let db = &ctx.db;
    let now = now_rfc3339();

    // TT2:queued → running
    let mut a: tasks::ActiveModel = task.clone().into();
    a.status = Set(TaskStatus::Running);
    a.started_at = Set(Some(now.clone()));
    a.updated_at = Set(now);
    let task = a.update(db).await?;
    emit_task(ctx, &task, TaskStatus::Running);

    log(
        ctx,
        &task.id,
        "info",
        &format!("开始执行 {} 任务", task_type_label(task.task_type)),
    )
    .await?;

    let result = match (cert.issuance_method, task.task_type) {
        (IssuanceMethod::SelfSigned, TaskType::Issue) => {
            self_signed::run_issue_self_signed(ctx, &task, &cert)
                .await
                .map(ExecOutcome::Done)
        }
        (IssuanceMethod::SelfSigned, TaskType::Renew) => {
            self_signed::run_renew_self_signed(ctx, &task, &cert)
                .await
                .map(ExecOutcome::Done)
        }
        (IssuanceMethod::SelfSigned, TaskType::Revoke) => {
            self_signed::run_revoke_self_signed(ctx, &task, &cert)
                .await
                .map(ExecOutcome::Done)
        }
        // acme issue/renew 同一 order 流程(首签/续签不区分,acme DA5);可能挂起等待手动(DNS-01)。
        (IssuanceMethod::Acme, TaskType::Issue | TaskType::Renew) => {
            acme::run_acme_issue_or_renew(ctx, &task, &cert).await
        }
        (IssuanceMethod::Acme, TaskType::Revoke) => acme::run_revoke_acme(ctx, &task, &cert)
            .await
            .map(ExecOutcome::Done),
    };

    match result {
        Ok(ExecOutcome::Done(summary)) => succeed_task(ctx, &task, &summary).await?,
        // DNS-01 挂起:任务保持 `running`(让出 worker,不置终态),等 confirm 续推。
        Ok(ExecOutcome::Suspended) => {
            log(
                ctx,
                &task.id,
                "info",
                "DNS-01 挑战等待手动添加 TXT,任务挂起(worker 已让出),等确认后续推",
            )
            .await?;
        }
        Err(e) => {
            // 驱动证书失败态(证书状态机唯一真相)
            drive_cert_failure(ctx, &cert, task.task_type, &e.message).await?;
            fail_task(ctx, &task, &e.message).await?;
        }
    }
    // 一次执行完成 → 证书可能进/出待处理集合,发红点合并信号。
    dashboard::emit_changed(ctx).await;
    Ok(())
}

async fn drive_cert_failure(
    ctx: &CoreContext,
    cert: &certificates::Model,
    task_type: TaskType,
    message: &str,
) -> CoreResult<()> {
    let new_status = match task_type {
        TaskType::Issue => CertificateStatus::IssueFailed,
        TaskType::Revoke => CertificateStatus::Valid,
        TaskType::Renew => CertificateStatus::RenewalFailed,
    };
    certificates::ActiveModel {
        id: Set(cert.id.clone()),
        status: Set(new_status),
        last_error: Set(Some(message.to_string())),
        updated_at: Set(now_rfc3339()),
        ..Default::default()
    }
    .update(&ctx.db)
    .await?;
    emit_cert(ctx, &cert.id, new_status);
    Ok(())
}

/// 仅更新证书状态 + updated_at(过渡态推进用;不动其他列)。
async fn update_cert_status(
    ctx: &CoreContext,
    cert_id: &str,
    status: CertificateStatus,
) -> CoreResult<()> {
    certificates::ActiveModel {
        id: Set(cert_id.to_string()),
        status: Set(status),
        updated_at: Set(now_rfc3339()),
        ..Default::default()
    }
    .update(&ctx.db)
    .await?;
    emit_cert(ctx, cert_id, status);
    Ok(())
}

/// TT3:running → succeeded。
async fn succeed_task(ctx: &CoreContext, task: &tasks::Model, summary: &str) -> CoreResult<()> {
    log(ctx, &task.id, "info", summary).await?;
    let now = now_rfc3339();
    let mut a: tasks::ActiveModel = task.clone().into();
    a.status = Set(TaskStatus::Succeeded);
    a.finished_at = Set(Some(now.clone()));
    a.result_summary = Set(Some(summary.to_string()));
    a.updated_at = Set(now);
    let task = a.update(&ctx.db).await?;
    emit_task(ctx, &task, TaskStatus::Succeeded);
    Ok(())
}

/// TT4:running → failed(失败任务保持 failed,重试派生新任务,不回炉)。
async fn fail_task(ctx: &CoreContext, task: &tasks::Model, reason: &str) -> CoreResult<()> {
    log(ctx, &task.id, "error", reason).await?;
    let now = now_rfc3339();
    let mut a: tasks::ActiveModel = task.clone().into();
    a.status = Set(TaskStatus::Failed);
    a.finished_at = Set(Some(now.clone()));
    a.failure_reason = Set(Some(reason.to_string()));
    a.result_summary = Set(Some("执行失败".to_string()));
    a.updated_at = Set(now);
    let task = a.update(&ctx.db).await?;
    emit_task(ctx, &task, TaskStatus::Failed);
    Ok(())
}

/// 追加一条执行日志(seq 自增,单 worker 顺序稳定)。**脱敏**:message 绝不含密钥材料(AR4/L6)。
/// 追加后发 `task_log_appended`(前端据 seq 增量拉 `?afterSeq`)。
async fn log(ctx: &CoreContext, task_id: &str, level: &str, message: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let seq = task_log_entries::Entity::find()
        .filter(task_log_entries::Column::TaskId.eq(task_id))
        .count(db)
        .await? as i32
        + 1;
    task_log_entries::ActiveModel {
        id: Set(new_id()),
        task_id: Set(task_id.to_string()),
        seq: Set(seq),
        logged_at: Set(now_rfc3339()),
        level: Set(level.to_string()),
        message: Set(message.to_string()),
    }
    .insert(db)
    .await?;
    ctx.emit(DomainEvent::TaskLogAppended {
        task_id: task_id.to_string(),
        seq,
    });
    Ok(())
}

/// 发 `certificate_status_changed`(证书状态机唯一真相在 core,事件仅为失效信号)。
fn emit_cert(ctx: &CoreContext, cert_id: &str, status: CertificateStatus) {
    ctx.emit(DomainEvent::CertificateStatusChanged {
        certificate_id: cert_id.to_string(),
        status,
    });
}

/// 发 `task_status_changed`(payload:taskId + 关联 certificateId + 新状态)。
fn emit_task(ctx: &CoreContext, task: &tasks::Model, status: TaskStatus) {
    ctx.emit(DomainEvent::TaskStatusChanged {
        task_id: task.id.clone(),
        certificate_id: task.certificate_id.clone(),
        status,
    });
}

/// 证书 SAN 域名 hostname 列表。
async fn san_hostnames(db: &DatabaseConnection, cert_id: &str) -> CoreResult<Vec<String>> {
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

fn task_type_label(t: TaskType) -> &'static str {
    match t {
        TaskType::Issue => "签发",
        TaskType::Renew => "续签",
        TaskType::Revoke => "吊销",
    }
}
