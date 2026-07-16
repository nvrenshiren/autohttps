//! 任务执行器(tokio worker)—— 持久队列消费者(AR5 / 决策7 / ARCHITECTURE §6.1)。
//!
//! `tasks.status=queued` 行即待办队列;单进程单 worker 按 `queued_at` FIFO 取出,
//! `queued→running→succeeded/failed`(TT2–TT4),据结果驱动证书状态机(唯一真相在 core,不复述)。
//!
//! **本切片范围**:承接 `self_signed` 的 `issue`(T2→T3/T4)、`renew`(T12/T13,经原根 CA 重签、
//! 刷新同一行 serial/有效期,不新建实体 DC1)与 `revoke`(T18/T19);**acme 尚未接入**(留后续),
//! 遇到 acme 证书即**跳过**(保持 queued,现状打桩)。日志脱敏(AR4/L6):`task_log_entries.message`
//! **绝不含任何密钥材料**。

use crate::ca;
use crate::domain::enums::{CertificateStatus, IssuanceMethod, RootCaStatus, TaskStatus, TaskType};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::{
    certificate_domains, certificates, domains, internal_cert_revocations, root_cas,
    task_log_entries, tasks,
};
use crate::services::context::CoreContext;
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;
use std::time::Duration;

/// 内网叶子证书默认有效期(天)。浏览器对服务器证书有效期上限约 398 天,取 365 稳妥。
const LEAF_VALIDITY_DAYS: i64 = 365;
/// 空闲轮询间隔(无可执行任务时休眠)。
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// 单次 tick 扫描的 queued 批量上限(挑第一个本切片可执行者)。
const SCAN_BATCH: u64 = 50;

/// 启动后台执行器循环(server/desktop boot 之后调用)。返回 JoinHandle(通常无需 join,自行常驻)。
pub fn spawn(ctx: CoreContext) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("任务执行器已启动(轮询 self_signed issue/revoke)");
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

/// 处理一个可执行任务;返回是否处理了任务(`false`=当前无本切片可执行任务,应休眠)。
///
/// 可执行 = 关联证书存在且为 `self_signed`(issue/renew/revoke 三类均承接)。acme 证书保持 `queued`。
pub async fn tick(ctx: &CoreContext) -> CoreResult<bool> {
    let db = &ctx.db;
    let queued = tasks::Entity::find()
        .filter(tasks::Column::Status.eq(TaskStatus::Queued))
        .order_by_asc(tasks::Column::QueuedAt)
        .limit(SCAN_BATCH)
        .all(db)
        .await?;

    for task in queued {
        let cert = certificates::Entity::find_by_id(&task.certificate_id).one(db).await?;
        let Some(cert) = cert else {
            // 证书已删除(删除会清理未完成任务;此为兜底)→ 置失败,不再纠缠
            fail_task(ctx, &task, "关联证书已删除,任务无法执行").await?;
            return Ok(true);
        };
        // 本切片仅 self_signed;acme 保持 queued(现状打桩)
        if cert.issuance_method != IssuanceMethod::SelfSigned {
            continue;
        }
        claim_and_run(ctx, task, cert).await?;
        return Ok(true);
    }
    Ok(false)
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

    log(ctx, &task.id, "info", &format!("开始执行 {} 任务", task_type_label(task.task_type))).await?;

    let result = match task.task_type {
        TaskType::Issue => run_issue_self_signed(ctx, &task, &cert).await,
        TaskType::Renew => run_renew_self_signed(ctx, &task, &cert).await,
        TaskType::Revoke => run_revoke_self_signed(ctx, &task, &cert).await,
    };

    match result {
        Ok(summary) => succeed_task(ctx, &task, &summary).await,
        Err(e) => {
            // 驱动证书失败态(证书状态机唯一真相)
            drive_cert_failure(ctx, &cert, task.task_type, &e.message).await?;
            fail_task(ctx, &task, &e.message).await
        }
    }
}

/// self_signed 签发(T2 issuing → T3 valid):用指定根 CA 签发叶子(SAN=证书关联域名)。
async fn run_issue_self_signed(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<String> {
    let db = &ctx.db;

    // T2:pending_issue → issuing
    update_cert_status(ctx, &cert.id, CertificateStatus::Issuing).await?;
    log(ctx, &task.id, "info", "证书置为签发中").await?;

    // 根 CA(仍须 active)
    let root_ca_id = cert
        .root_ca_id
        .clone()
        .ok_or_else(|| CoreError::internal("self_signed 证书缺少根 CA 引用"))?;
    let root_ca = root_cas::Entity::find_by_id(&root_ca_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::InvalidRootCaReference, "根 CA 不存在"))?;
    if root_ca.status != RootCaStatus::Active {
        return Err(CoreError::new(ErrorCode::RootCaExpired, "根 CA 已过期,拒绝签发"));
    }
    log(ctx, &task.id, "info", &format!("使用根 CA「{}」签发", root_ca.name)).await?;

    // SAN 域名
    let hostnames = san_hostnames(db, &cert.id).await?;
    if hostnames.is_empty() {
        return Err(CoreError::internal("证书无关联域名(SAN 为空),无法签发"));
    }
    log(ctx, &task.id, "info", &format!("SAN: {}", hostnames.join(", "))).await?;

    // 读根 CA 私钥(age 解密)→ 签发叶子(私钥材料仅在内存,不入日志)
    let root_key_pem = String::from_utf8(ctx.secrets.load(&root_ca.private_key_ref)?)
        .map_err(|_| CoreError::internal("根 CA 私钥材料损坏"))?;
    let leaf = ca::sign_leaf(&root_ca.cert_pem, &root_key_pem, &hostnames, LEAF_VALIDITY_DAYS)?;
    log(
        ctx,
        &task.id,
        "info",
        &format!("叶子证书已签发,序列号 {},有效期至 {}", leaf.serial_number, leaf.not_after),
    )
    .await?;

    // 落地材料:公开证书 + 私钥密文落数据目录,库内只存 ref(AR4)
    let cert_ref = new_id();
    ctx.secrets.store(&cert_ref, leaf.cert_pem.as_bytes())?;
    let key_ref = new_id();
    ctx.secrets.store(&key_ref, leaf.key_pem.as_bytes())?;

    // T3:issuing → valid(写标识/有效期/引用,清 last_error)
    let now = now_rfc3339();
    certificates::ActiveModel {
        id: Set(cert.id.clone()),
        status: Set(CertificateStatus::Valid),
        serial_number: Set(Some(leaf.serial_number.clone())),
        fingerprint: Set(Some(leaf.fingerprint)),
        not_before: Set(Some(leaf.not_before)),
        not_after: Set(Some(leaf.not_after)),
        issued_at: Set(Some(now.clone())),
        cert_pem_ref: Set(Some(cert_ref)),
        private_key_ref: Set(Some(key_ref)),
        last_error: Set(None),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await?;

    Ok(format!("签发成功(序列号 {})", leaf.serial_number))
}

/// self_signed 续签(T12 renewing → valid):经**原根 CA**重签叶子(换新私钥),**刷新同一证书行**的
/// serial/指纹/有效期/文件引用(不新建实体,DC1)。旧文件材料随之清理。
async fn run_renew_self_signed(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<String> {
    let db = &ctx.db;

    // 保持/确认 renewing(证书服务发起时已置;此处幂等,兼容重试链直达)
    update_cert_status(ctx, &cert.id, CertificateStatus::Renewing).await?;
    log(ctx, &task.id, "info", "证书置为续签中").await?;

    // 原根 CA(仍须 active)
    let root_ca_id = cert
        .root_ca_id
        .clone()
        .ok_or_else(|| CoreError::internal("self_signed 证书缺少根 CA 引用"))?;
    let root_ca = root_cas::Entity::find_by_id(&root_ca_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::InvalidRootCaReference, "根 CA 不存在"))?;
    if root_ca.status != RootCaStatus::Active {
        return Err(CoreError::new(ErrorCode::RootCaExpired, "根 CA 已过期,拒绝续签"));
    }
    log(ctx, &task.id, "info", &format!("经原根 CA「{}」重签", root_ca.name)).await?;

    let hostnames = san_hostnames(db, &cert.id).await?;
    if hostnames.is_empty() {
        return Err(CoreError::internal("证书无关联域名(SAN 为空),无法续签"));
    }

    // 重签叶子(rcgen 每次生成新密钥 → 换新私钥,呼应 T20 语义)
    let root_key_pem = String::from_utf8(ctx.secrets.load(&root_ca.private_key_ref)?)
        .map_err(|_| CoreError::internal("根 CA 私钥材料损坏"))?;
    let leaf = ca::sign_leaf(&root_ca.cert_pem, &root_key_pem, &hostnames, LEAF_VALIDITY_DAYS)?;
    log(
        ctx,
        &task.id,
        "info",
        &format!("续签完成,新序列号 {},有效期至 {}", leaf.serial_number, leaf.not_after),
    )
    .await?;

    // 落新材料 → 记旧引用备清理
    let old_cert_ref = cert.cert_pem_ref.clone();
    let old_key_ref = cert.private_key_ref.clone();
    let cert_ref = new_id();
    ctx.secrets.store(&cert_ref, leaf.cert_pem.as_bytes())?;
    let key_ref = new_id();
    ctx.secrets.store(&key_ref, leaf.key_pem.as_bytes())?;

    // T12:renewing → valid(刷新同一行标识/有效期/引用,清 last_error;DC1 不新建实体)
    let now = now_rfc3339();
    certificates::ActiveModel {
        id: Set(cert.id.clone()),
        status: Set(CertificateStatus::Valid),
        serial_number: Set(Some(leaf.serial_number.clone())),
        fingerprint: Set(Some(leaf.fingerprint)),
        not_before: Set(Some(leaf.not_before)),
        not_after: Set(Some(leaf.not_after)),
        issued_at: Set(Some(now.clone())),
        cert_pem_ref: Set(Some(cert_ref)),
        private_key_ref: Set(Some(key_ref)),
        last_error: Set(None),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await?;

    // 清理旧文件材料(避免孤儿密文;换新私钥后旧私钥应销毁)
    if let Some(r) = old_cert_ref {
        let _ = ctx.secrets.remove(&r);
    }
    if let Some(r) = old_key_ref {
        let _ = ctx.secrets.remove(&r);
    }

    Ok(format!("续签成功(新序列号 {})", leaf.serial_number))
}

/// self_signed 吊销(T18 revoking → revoked):根 CA 记本地作废 + 证书转 revoked。
async fn run_revoke_self_signed(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<String> {
    let db = &ctx.db;

    let root_ca_id = cert
        .root_ca_id
        .clone()
        .ok_or_else(|| CoreError::internal("self_signed 证书缺少根 CA 引用"))?;
    let serial = cert
        .serial_number
        .clone()
        .ok_or_else(|| CoreError::internal("证书无序列号,无法记作废"))?;
    let now = now_rfc3339();

    // 写作废记录(rcgen 无 CRL/OCSP,MVP 本地作废记录);(root_ca_id, serial) 唯一 → 幂等
    let existing = internal_cert_revocations::Entity::find()
        .filter(internal_cert_revocations::Column::RootCaId.eq(&root_ca_id))
        .filter(internal_cert_revocations::Column::SerialNumber.eq(&serial))
        .one(db)
        .await?;
    if existing.is_none() {
        internal_cert_revocations::ActiveModel {
            id: Set(new_id()),
            root_ca_id: Set(root_ca_id),
            serial_number: Set(serial.clone()),
            certificate_id: Set(Some(cert.id.clone())),
            revoked_at: Set(now.clone()),
            created_at: Set(now.clone()),
        }
        .insert(db)
        .await?;
    }
    log(ctx, &task.id, "info", &format!("已在根 CA 记录作废,序列号 {serial}")).await?;

    // T18:revoking → revoked
    update_cert_status(ctx, &cert.id, CertificateStatus::Revoked).await?;

    Ok(format!("已吊销(序列号 {serial})"))
}

/// 失败时驱动证书回退态(证书状态机唯一真相):
/// - issue 失败 → `issue_failed`(T4);
/// - revoke 失败 → 回退 `valid`(T19 近似:自签本地作废几乎不失败,回退取最常见发起前态)。
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
    a.update(&ctx.db).await?;
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
    a.update(&ctx.db).await?;
    Ok(())
}

/// 追加一条执行日志(seq 自增,单 worker 顺序稳定)。**脱敏**:message 绝不含密钥材料(AR4/L6)。
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
    Ok(())
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
