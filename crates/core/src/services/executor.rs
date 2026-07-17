//! 任务执行器(tokio worker)—— 持久队列消费者(AR5 / 决策7 / ARCHITECTURE §6.1)。
//!
//! `tasks.status=queued` 行即待办队列;单进程单 worker 按 `queued_at` FIFO 取出,
//! `queued→running→succeeded/failed`(TT2–TT4),据结果驱动证书状态机(唯一真相在 core,不复述)。
//!
//! **本切片范围**:承接 `self_signed` 的 `issue`(T2→T3/T4)、`renew`(T12/T13,经原根 CA 重签、
//! 刷新同一行 serial/有效期,不新建实体 DC1)与 `revoke`(T18/T19);**acme 的 `issue`/`renew`
//! (HTTP-01 + DNS-01 手动)与 `revoke`** 经 instant-acme 跑通:
//! - issue/renew:建单→每域名取挑战→HTTP-01 放 webroot 文件通知就绪 / DNS-01 计算 TXT 置
//!   `awaiting_manual` **挂起等待手动**→(全部就绪后)轮询→finalize→取证(renew 刷新同一行 DC1)。
//! - revoke:向 CA 发吊销请求(revoke_cert)→ 证书 `revoking→revoked`(T18)。
//! **挂起等待手动**(DNS-01):任务遇 `awaiting_manual` 即让出 worker(任务留 `running`,不占 worker);
//! `POST /acme/challenges/{id}/confirm` 重建订单通知 CA 校验、全部通过后续推 finalize(见 [`confirm_challenge`])。
//! 日志脱敏(AR4/L6):`task_log_entries.message` **绝不含任何密钥材料**。

use crate::ca;
use crate::domain::enums::{
    AcmeAccountStatus, CertificateStatus, ChallengeStatus, IssuanceMethod, RootCaStatus, TaskStatus,
    TaskType, ValidationMethod,
};
use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    acme_accounts, certificate_domains, certificates, challenges, domains,
    http01_validation_configs, internal_cert_revocations, root_cas, task_log_entries, tasks,
};
use crate::services::acme;
use crate::services::context::CoreContext;
use crate::services::dashboard;
use crate::util::{new_id, now_rfc3339};
use instant_acme::{
    ChallengeType, Identifier, NewOrder, Order, OrderStatus, RetryPolicy, RevocationRequest,
};
use rustls_pki_types::CertificateDer;
use sea_orm::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 单个可执行任务的执行结果:完成(→ 成功任务)或挂起等待手动(DNS-01,任务留 `running`)。
enum ExecOutcome {
    /// 执行完成,携带任务结果摘要 → `running → succeeded`(TT3)。
    Done(String),
    /// DNS-01 挑战进入 `awaiting_manual`,任务让出 worker、保持 `running`,等 confirm 续推(不卡死)。
    Suspended,
}

/// 内网叶子证书默认有效期(天)。浏览器对服务器证书有效期上限约 398 天,取 365 稳妥。
const LEAF_VALIDITY_DAYS: i64 = 365;
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
    let cert = certificates::Entity::find_by_id(&task.certificate_id).one(db).await?;
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

    log(ctx, &task.id, "info", &format!("开始执行 {} 任务", task_type_label(task.task_type))).await?;

    let result = match (cert.issuance_method, task.task_type) {
        (IssuanceMethod::SelfSigned, TaskType::Issue) => {
            run_issue_self_signed(ctx, &task, &cert).await.map(ExecOutcome::Done)
        }
        (IssuanceMethod::SelfSigned, TaskType::Renew) => {
            run_renew_self_signed(ctx, &task, &cert).await.map(ExecOutcome::Done)
        }
        (IssuanceMethod::SelfSigned, TaskType::Revoke) => {
            run_revoke_self_signed(ctx, &task, &cert).await.map(ExecOutcome::Done)
        }
        // acme issue/renew 同一 order 流程(首签/续签不区分,acme DA5);可能挂起等待手动(DNS-01)。
        (IssuanceMethod::Acme, TaskType::Issue | TaskType::Renew) => {
            run_acme_issue_or_renew(ctx, &task, &cert).await
        }
        (IssuanceMethod::Acme, TaskType::Revoke) => {
            run_revoke_acme(ctx, &task, &cert).await.map(ExecOutcome::Done)
        }
    };

    match result {
        Ok(ExecOutcome::Done(summary)) => succeed_task(ctx, &task, &summary).await?,
        // DNS-01 挂起:任务保持 `running`(让出 worker,不置终态),等 confirm 续推。
        Ok(ExecOutcome::Suspended) => {
            log(ctx, &task.id, "info", "DNS-01 挑战等待手动添加 TXT,任务挂起(worker 已让出),等确认后续推")
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
    emit_cert(ctx, &cert.id, CertificateStatus::Valid);

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
    emit_cert(ctx, &cert.id, CertificateStatus::Valid);

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

/// acme 签发/续签(同一 order 流程,首签/续签不区分 acme DA5)。issue:T2 issuing→T3 valid;
/// renew:T12 renewing→valid(刷新同一行 serial/有效期,不新建实体 DC1)。经 instant-acme:建单 →
/// 每域名取挑战(HTTP-01 放 webroot 文件、通知就绪 CT2;DNS-01 计算 TXT 置 `awaiting_manual` CT3
/// **挂起等待手动**)→ 全部就绪则 finalize 取证;任一域名 DNS-01 挂起则 `ExecOutcome::Suspended`
/// (任务让出 worker,confirm 续推)。任一失败整体失败交回 issue_failed/renewal_failed(§3.4)。
async fn run_acme_issue_or_renew(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<ExecOutcome> {
    let db = &ctx.db;
    let is_renew = matches!(task.task_type, TaskType::Renew);

    // T2/T12:进行中态(issue→issuing / renew→renewing;renew 幂等确认,兼容重试链直达)
    let in_progress =
        if is_renew { CertificateStatus::Renewing } else { CertificateStatus::Issuing };
    update_cert_status(ctx, &cert.id, in_progress).await?;
    log(ctx, &task.id, "info", if is_renew { "证书置为续签中(ACME)" } else { "证书置为签发中(ACME)" })
        .await?;

    // ACME 账户(须 registered、有账户密钥)
    let account_row = load_registered_account(ctx, cert).await?;
    log(ctx, &task.id, "info", &format!("使用 ACME 账户「{}」", account_row.contact_email)).await?;

    // SAN 域名(需 id/hostname/验证方式;HTTP-01 与 DNS-01 均支持)
    let san = acme_san_domains(db, &cert.id).await?;
    if san.is_empty() {
        return Err(CoreError::internal("证书无关联域名(SAN 为空),无法签发"));
    }
    for d in &san {
        if d.validation_method.is_none() {
            return Err(CoreError::new(
                ErrorCode::DomainValidationMethodRequired,
                format!("域名 {} 未设置验证方式", d.hostname),
            ));
        }
    }
    let hostnames: Vec<String> = san.iter().map(|d| d.hostname.clone()).collect();
    log(ctx, &task.id, "info", &format!("SAN: {}", hostnames.join(", "))).await?;

    // 载入账户 + 建订单(SAN 域名)。捕获 order URL 存挑战 `authorization_url`,供 confirm/retry 重建订单。
    let acme_account = acme::load_acme_account(ctx, &account_row).await?;
    let identifiers: Vec<Identifier> =
        hostnames.iter().map(|h| Identifier::Dns(h.clone())).collect();
    let mut order =
        acme_account.new_order(&NewOrder::new(&identifiers)).await.map_err(acme::map_acme_err)?;
    let order_url = order.url().to_string();
    log(ctx, &task.id, "info", "已向 CA 建立订单").await?;

    // 每域名:建挑战记录(pending CT1)→ HTTP-01 放文件+通知就绪(validating CT2)/ DNS-01 计算 TXT
    // 置 awaiting_manual(CT3,挂起)。任一域名进入 awaiting_manual → 本次整体挂起。
    let mut any_awaiting = false;
    {
        let mut authorizations = order.authorizations();
        while let Some(result) = authorizations.next().await {
            let mut authz = result.map_err(acme::map_acme_err)?;
            let identifier = authz.identifier().to_string();
            let domain = san
                .iter()
                .find(|d| d.hostname == identifier)
                .ok_or_else(|| CoreError::internal(format!("授权域名 {identifier} 不在证书 SAN 中")))?;

            match domain.validation_method {
                Some(ValidationMethod::Http01) => {
                    let Some(mut challenge) = authz.challenge(ChallengeType::Http01) else {
                        return Err(CoreError::internal(format!("域名 {identifier} 无 HTTP-01 挑战")));
                    };
                    let token = challenge.token.clone();
                    let key_auth = challenge.key_authorization().as_str().to_string();
                    let needs_ready =
                        matches!(challenge.status, instant_acme::ChallengeStatus::Pending);

                    // 放验证文件(always-valid 下可达性不重要,仍走流程);清理由 finalize 依 token 重算路径。
                    let webroot = resolve_webroot(ctx, db, &domain.id).await?;
                    write_challenge_file(&webroot, &token, &key_auth)?;
                    let url_path = format!("/.well-known/acme-challenge/{token}");

                    let challenge_id = insert_challenge(
                        ctx,
                        &task.id,
                        &domain.id,
                        ValidationMethod::Http01,
                        ChallengeFields {
                            http_file_path: Some(url_path),
                            http_file_content: Some(key_auth),
                            authorization_url: Some(order_url.clone()),
                            ..Default::default()
                        },
                    )
                    .await?;
                    log(ctx, &task.id, "info", &format!("域名 {identifier}:HTTP-01 验证文件已放置")).await?;

                    if needs_ready {
                        challenge.set_ready().await.map_err(acme::map_acme_err)?;
                    }
                    update_challenge_status(
                        ctx,
                        &challenge_id,
                        &task.id,
                        &domain.id,
                        ChallengeStatus::Validating,
                        None,
                    )
                    .await?;
                }
                Some(ValidationMethod::Dns01) => {
                    let Some(challenge) = authz.challenge(ChallengeType::Dns01) else {
                        return Err(CoreError::internal(format!("域名 {identifier} 无 DNS-01 挑战")));
                    };
                    // DNS-01 待添加 TXT:记录名 `_acme-challenge.<base>`(通配符去 `*.`),值为 key_auth 摘要。
                    let txt_name = dns_txt_name(&domain.hostname);
                    let txt_value = challenge.key_authorization().dns_value();

                    let challenge_id = insert_challenge(
                        ctx,
                        &task.id,
                        &domain.id,
                        ValidationMethod::Dns01,
                        ChallengeFields {
                            dns_txt_name: Some(txt_name.clone()),
                            dns_txt_value: Some(txt_value),
                            authorization_url: Some(order_url.clone()),
                            ..Default::default()
                        },
                    )
                    .await?;
                    // CT3:pending → awaiting_manual(不 set_ready,挂起等用户加 TXT 后 confirm)
                    update_challenge_status(
                        ctx,
                        &challenge_id,
                        &task.id,
                        &domain.id,
                        ChallengeStatus::AwaitingManual,
                        None,
                    )
                    .await?;
                    log(
                        ctx,
                        &task.id,
                        "info",
                        &format!("域名 {identifier}:DNS-01 待手动添加 TXT 记录 {txt_name},等待确认"),
                    )
                    .await?;
                    any_awaiting = true;
                }
                None => unreachable!("上文已校验验证方式非空"),
            }
        }
    }

    // 任一域名 DNS-01 挂起 → 整体挂起(任务让出 worker,confirm 续推 finalize)。
    if any_awaiting {
        return Ok(ExecOutcome::Suspended);
    }

    // 全部就绪(纯 HTTP-01)→ 轮询 + finalize + 取证 + 落证书(finalize 内含失败时挑战置 failed)。
    let summary = finalize_acme_order(ctx, task, cert, &mut order).await?;
    Ok(ExecOutcome::Done(summary))
}

/// 驱动一个已就绪(全部挑战已通知 CA)的 ACME 订单到取证并落证书(issue/renew 共用,confirm 续推亦用)。
///
/// 轮询订单至 ready → 全部挑战置 passed(CT5)→ finalize(自动生成叶子密钥+CSR)→ 取证(链 PEM)→
/// 清理 HTTP-01 验证文件 → 落材料 + 刷新证书行至 valid(renew 清旧文件材料,DC1)。失败时把本任务
/// 全部挑战置 failed(CT6)并回传 Err(上层驱动证书失败态)。返回任务结果摘要。
async fn finalize_acme_order(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
    order: &mut Order,
) -> CoreResult<String> {
    let db = &ctx.db;
    let poll = RetryPolicy::default();

    // 轮询订单至 ready(Pebble always-valid 下快速通过)
    let ready = match order.poll_ready(&poll).await {
        Ok(status) => status,
        Err(e) => {
            let err = acme::map_acme_err(e);
            fail_task_challenges(ctx, &task.id, "订单验证失败").await;
            return Err(err);
        }
    };
    if ready != OrderStatus::Ready {
        fail_task_challenges(ctx, &task.id, "挑战未通过").await;
        return Err(CoreError::internal(format!("订单验证未通过(状态 {ready:?})")));
    }

    // 全部域名验证通过(CT5,passed)
    let ch_rows = task_challenges(db, &task.id).await?;
    for ch in &ch_rows {
        update_challenge_status(ctx, &ch.id, &task.id, &ch.domain_id, ChallengeStatus::Passed, None)
            .await?;
    }
    log(ctx, &task.id, "info", "全部域名验证通过").await?;

    // finalize(返回叶子私钥 PEM)→ 取证(链 PEM:叶子+中间)
    let leaf_key_pem = match order.finalize().await {
        Ok(k) => k,
        Err(e) => {
            let err = acme::map_acme_err(e);
            fail_task_challenges(ctx, &task.id, "取证失败").await;
            return Err(err);
        }
    };
    let chain_pem = match order.poll_certificate(&poll).await {
        Ok(c) => c,
        Err(e) => {
            let err = acme::map_acme_err(e);
            fail_task_challenges(ctx, &task.id, "取证失败").await;
            return Err(err);
        }
    };
    log(ctx, &task.id, "info", "已从 CA 取得证书").await?;

    // 清理 HTTP-01 验证文件(成功/失败均清理,flows §4.2 步骤4;依 token 重算路径)
    cleanup_http_challenge_files(ctx, db, &ch_rows).await;

    // 解析叶子标识/有效期(链首块为叶子)
    let meta = ca::parse_leaf_metadata(&chain_pem)?;
    log(
        ctx,
        &task.id,
        "info",
        &format!("叶子证书序列号 {},有效期至 {}", meta.serial_number, meta.not_after),
    )
    .await?;

    // 落材料:证书链(公开)+ 叶子私钥(敏感 AR4)密文落盘,库内只存 ref
    let old_cert_ref = cert.cert_pem_ref.clone();
    let old_key_ref = cert.private_key_ref.clone();
    let cert_ref = new_id();
    ctx.secrets.store(&cert_ref, chain_pem.as_bytes())?;
    let key_ref = new_id();
    ctx.secrets.store(&key_ref, leaf_key_pem.as_bytes())?;

    // T3/T12:进行中 → valid(刷新同一行标识/有效期/引用,清 last_error;DC1 不新建实体)
    let now = now_rfc3339();
    certificates::ActiveModel {
        id: Set(cert.id.clone()),
        status: Set(CertificateStatus::Valid),
        serial_number: Set(Some(meta.serial_number.clone())),
        fingerprint: Set(Some(meta.fingerprint)),
        not_before: Set(Some(meta.not_before)),
        not_after: Set(Some(meta.not_after)),
        issued_at: Set(Some(now.clone())),
        cert_pem_ref: Set(Some(cert_ref)),
        private_key_ref: Set(Some(key_ref)),
        last_error: Set(None),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await?;
    emit_cert(ctx, &cert.id, CertificateStatus::Valid);

    // renew:清理旧文件材料(issue 时旧 ref 为空,天然跳过)
    if let Some(r) = old_cert_ref {
        let _ = ctx.secrets.remove(&r);
    }
    if let Some(r) = old_key_ref {
        let _ = ctx.secrets.remove(&r);
    }

    let verb = if matches!(task.task_type, TaskType::Renew) { "续签" } else { "签发" };
    Ok(format!("{verb}成功(序列号 {})", meta.serial_number))
}

/// acme 吊销(T18 revoking → revoked)。向 CA 发吊销请求(revoke_cert):取证书链叶子 DER →
/// `Account::revoke`。成功后证书 `revoking→revoked`。失败上层回退证书(T19,drive_cert_failure)。
async fn run_revoke_acme(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<String> {
    // 保持/确认 revoking(证书服务发起时已置)
    update_cert_status(ctx, &cert.id, CertificateStatus::Revoking).await?;
    log(ctx, &task.id, "info", "证书置为吊销中(ACME)").await?;

    // 账户(须 registered)+ 证书链叶子 DER
    let account_row = load_registered_account(ctx, cert).await?;
    let cert_ref = cert
        .cert_pem_ref
        .as_deref()
        .ok_or_else(|| CoreError::internal("证书无本地文件,无法向 CA 吊销"))?;
    let chain_pem = String::from_utf8(ctx.secrets.load(cert_ref)?)
        .map_err(|_| CoreError::internal("证书文件材料损坏"))?;
    let leaf_der = ca::leaf_der_from_chain(&chain_pem)?;

    // 向 CA 发吊销请求(reason 缺省;instant-acme 用账户密钥授权)
    let acme_account = acme::load_acme_account(ctx, &account_row).await?;
    let certificate = CertificateDer::from(leaf_der);
    let req = RevocationRequest { certificate: &certificate, reason: None };
    acme_account.revoke(&req).await.map_err(acme::map_acme_err)?;
    log(ctx, &task.id, "info", "已向 CA 提交吊销请求").await?;

    // T18:revoking → revoked
    update_cert_status(ctx, &cert.id, CertificateStatus::Revoked).await?;

    let serial = cert.serial_number.clone().unwrap_or_default();
    Ok(format!("已吊销(序列号 {serial})"))
}

/// 载入证书关联的 ACME 账户行,校验为 `registered`(issue/renew/revoke 共用前置)。
async fn load_registered_account(
    ctx: &CoreContext,
    cert: &certificates::Model,
) -> CoreResult<acme_accounts::Model> {
    let account_id = cert
        .acme_account_id
        .clone()
        .ok_or_else(|| CoreError::new(ErrorCode::AcmeAccountRequired, "acme 证书缺少 ACME 账户"))?;
    let account_row = acme_accounts::Entity::find_by_id(&account_id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::InvalidAcmeAccountReference, "ACME 账户不存在"))?;
    if account_row.status != AcmeAccountStatus::Registered {
        return Err(CoreError::new(
            ErrorCode::AcmeAccountNotRegistered,
            "指定的 ACME 账户尚未注册成功",
        ));
    }
    Ok(account_row)
}

/// DNS-01 确认(去桩,CT4)——用户已加 TXT,通知 CA 校验;全部挑战不再等待手动即续推 finalize。
///
/// 单挑战 `awaiting_manual → validating`(set_ready 通知 CA);若本任务已无 `awaiting_manual`/`pending`
/// 挑战(全部就绪),重建订单驱动 finalize → 证书 valid、任务 succeeded;否则等其余挑战确认。
/// worker 不参与本流程(挂起任务停在 `running`),由本函数(api 请求线程)推进,不卡死执行器。
pub(crate) async fn confirm_challenge(ctx: &CoreContext, challenge_id: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let challenge = challenges::Entity::find_by_id(challenge_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::ChallengeNotFound, "挑战不存在"))?;
    if challenge.status != ChallengeStatus::AwaitingManual {
        return Err(CoreError::new(
            ErrorCode::ChallengeNotAwaitingManual,
            "仅等待手动配置的挑战可确认",
        ));
    }
    let (task, cert, account_row) = challenge_task_context(ctx, &challenge).await?;
    let hostname = domains::Entity::find_by_id(&challenge.domain_id)
        .one(db)
        .await?
        .map(|d| d.hostname)
        .ok_or_else(|| CoreError::internal("挑战关联域名已删除,无法确认"))?;
    let order_url = challenge
        .authorization_url
        .clone()
        .ok_or_else(|| CoreError::internal("挑战缺少订单引用,无法确认"))?;

    // 重建订单 → 定位本域名授权的 DNS-01 挑战 → 通知 CA 就绪(set_ready)。
    let acme_account = acme::load_acme_account(ctx, &account_row).await?;
    let mut order = acme_account.order(order_url).await.map_err(acme::map_acme_err)?;
    set_ready_for_domain(&mut order, &hostname, ChallengeType::Dns01).await?;

    // CT4:awaiting_manual → validating
    update_challenge_status(
        ctx,
        &challenge.id,
        &task.id,
        &challenge.domain_id,
        ChallengeStatus::Validating,
        None,
    )
    .await?;
    log(ctx, &task.id, "info", &format!("域名 {hostname}:已确认 TXT,通知 CA 校验")).await?;

    // 本任务是否仍有等待手动/待验证的挑战?有则等其余确认;无则续推 finalize。
    let pending = task_challenges(db, &task.id)
        .await?
        .into_iter()
        .filter(|c| {
            matches!(c.status, ChallengeStatus::AwaitingManual | ChallengeStatus::Pending)
        })
        .count();
    if pending > 0 {
        dashboard::emit_changed(ctx).await;
        return Ok(());
    }

    // 全部就绪 → 续推(仅当证书仍处进行中态,防重复 finalize)。
    if !matches!(cert.status, CertificateStatus::Issuing | CertificateStatus::Renewing) {
        dashboard::emit_changed(ctx).await;
        return Ok(());
    }
    match finalize_acme_order(ctx, &task, &cert, &mut order).await {
        Ok(summary) => succeed_task(ctx, &task, &summary).await?,
        Err(e) => {
            drive_cert_failure(ctx, &cert, task.task_type, &e.message).await?;
            fail_task(ctx, &task, &e.message).await?;
        }
    }
    dashboard::emit_changed(ctx).await;
    Ok(())
}

/// 挑战失败重试(去桩,CT7)。ACME 语义:失败挑战/订单不可原地复用,须**重建订单取新挑战**;
/// 故委派证书重试(certificates::retry:派生新 issue/renew 任务 → 执行器重建订单、建新挑战)。
/// 前置:挑战为 `failed`;其证书须处可重试态(issue_failed/renewal_failed),否则回传状态错误。
pub(crate) async fn retry_challenge(ctx: &CoreContext, challenge_id: &str) -> CoreResult<()> {
    let db = &ctx.db;
    let challenge = challenges::Entity::find_by_id(challenge_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::ChallengeNotFound, "挑战不存在"))?;
    if challenge.status != ChallengeStatus::Failed {
        return Err(CoreError::new(ErrorCode::ChallengeNotRetryable, "仅失败的挑战可重试"));
    }
    let task = tasks::Entity::find_by_id(&challenge.task_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::TaskNotFound, "挑战所属任务不存在"))?;
    let cert = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "挑战所属证书不存在"))?;
    if !cert.status.can_retry() {
        return Err(CoreError::new(
            ErrorCode::ChallengeNotRetryable,
            "该挑战对应的签发未处于可重试态(请对证书发起重试)",
        )
        .with_details(serde_json::json!({ "currentStatus": cert.status })));
    }
    // 委派证书重试:派生新任务 → 执行器重建订单、取新挑战(HTTP-01 重放文件 / DNS-01 重新展示 TXT)。
    crate::services::certificates::retry(ctx, &cert.id).await?;
    Ok(())
}

/// 载入挑战的执行上下文(任务 + 证书 + 已注册账户),confirm 用。
async fn challenge_task_context(
    ctx: &CoreContext,
    challenge: &challenges::Model,
) -> CoreResult<(tasks::Model, certificates::Model, acme_accounts::Model)> {
    let db = &ctx.db;
    let task = tasks::Entity::find_by_id(&challenge.task_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::TaskNotFound, "挑战所属任务不存在"))?;
    let cert = certificates::Entity::find_by_id(&task.certificate_id)
        .one(db)
        .await?
        .ok_or_else(|| CoreError::new(ErrorCode::CertNotFound, "挑战所属证书不存在"))?;
    let account_row = load_registered_account(ctx, &cert).await?;
    Ok((task, cert, account_row))
}

/// 在重建的订单中定位某域名授权对应类型的挑战并 `set_ready`(通知 CA 校验)。
async fn set_ready_for_domain(
    order: &mut Order,
    hostname: &str,
    r#type: ChallengeType,
) -> CoreResult<()> {
    let mut authorizations = order.authorizations();
    while let Some(result) = authorizations.next().await {
        let mut authz = result.map_err(acme::map_acme_err)?;
        if authz.identifier().to_string() != hostname {
            continue;
        }
        let mut challenge = authz
            .challenge(r#type.clone())
            .ok_or_else(|| CoreError::internal(format!("域名 {hostname} 无对应挑战")))?;
        challenge.set_ready().await.map_err(acme::map_acme_err)?;
        return Ok(());
    }
    Err(CoreError::internal(format!("订单中未找到域名 {hostname} 的授权")))
}

/// DNS-01 待添加 TXT 记录名:`_acme-challenge.<base>`;通配符 `*.example.com` 取 `example.com`。
fn dns_txt_name(hostname: &str) -> String {
    let base = hostname.strip_prefix("*.").unwrap_or(hostname);
    format!("_acme-challenge.{base}")
}

/// 证书 SAN 域名行(acme 需 hostname + 验证方式)。
async fn acme_san_domains(db: &DatabaseConnection, cert_id: &str) -> CoreResult<Vec<domains::Model>> {
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
    Ok(domains::Entity::find().filter(domains::Column::Id.is_in(ids)).all(db).await?)
}

/// 解析域名 HTTP-01 webroot:有配置用之;无则用数据目录下临时 webroot(DEA5 / 本切片兜底)。
async fn resolve_webroot(
    ctx: &CoreContext,
    db: &DatabaseConnection,
    domain_id: &str,
) -> CoreResult<PathBuf> {
    if let Some(cfg) = http01_validation_configs::Entity::find_by_id(domain_id).one(db).await? {
        Ok(PathBuf::from(cfg.webroot_path))
    } else {
        Ok(ctx.data_dir.join("acme-webroot").join(domain_id))
    }
}

/// 在 webroot 下写 `.well-known/acme-challenge/<token>` 验证文件,返回其文件系统路径。
fn write_challenge_file(webroot: &Path, token: &str, key_auth: &str) -> CoreResult<PathBuf> {
    let dir = webroot.join(".well-known").join("acme-challenge");
    std::fs::create_dir_all(&dir)
        .map_err(|e| CoreError::internal(format!("创建 webroot 目录失败: {e}")))?;
    let path = dir.join(token);
    std::fs::write(&path, key_auth.as_bytes())
        .map_err(|e| CoreError::internal(format!("写入验证文件失败: {e}")))?;
    Ok(path)
}

/// 挑战记录的可选展示/执行字段(HTTP-01 文件 / DNS-01 TXT / 订单引用)。
#[derive(Default)]
struct ChallengeFields {
    dns_txt_name: Option<String>,
    dns_txt_value: Option<String>,
    http_file_path: Option<String>,
    http_file_content: Option<String>,
    authorization_url: Option<String>,
}

/// 建挑战记录(CT1,pending)+ 发 `challenge_status_changed`。返回挑战 id。验证方式与字段由调用方给出
/// (HTTP-01 传文件路径/内容;DNS-01 传 TXT 名/值);`authorization_url` 存 order URL 供 confirm/retry 重建订单。
async fn insert_challenge(
    ctx: &CoreContext,
    task_id: &str,
    domain_id: &str,
    method: ValidationMethod,
    fields: ChallengeFields,
) -> CoreResult<String> {
    let id = new_id();
    let now = now_rfc3339();
    challenges::ActiveModel {
        id: Set(id.clone()),
        task_id: Set(task_id.to_string()),
        domain_id: Set(domain_id.to_string()),
        validation_method: Set(method),
        status: Set(ChallengeStatus::Pending),
        dns_txt_name: Set(fields.dns_txt_name),
        dns_txt_value: Set(fields.dns_txt_value),
        http_file_path: Set(fields.http_file_path),
        http_file_content: Set(fields.http_file_content),
        authorization_url: Set(fields.authorization_url),
        failed_reason: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(&ctx.db)
    .await?;
    emit_challenge(ctx, &id, task_id, domain_id, ChallengeStatus::Pending);
    Ok(id)
}

/// 本任务全部挑战行(finalize 标记 passed / 失败标记 failed / 清理文件用)。
async fn task_challenges(
    db: &DatabaseConnection,
    task_id: &str,
) -> CoreResult<Vec<challenges::Model>> {
    Ok(challenges::Entity::find()
        .filter(challenges::Column::TaskId.eq(task_id))
        .all(db)
        .await?)
}

/// 推进挑战状态(+ 可选失败原因)并发 `challenge_status_changed`。
async fn update_challenge_status(
    ctx: &CoreContext,
    challenge_id: &str,
    task_id: &str,
    domain_id: &str,
    status: ChallengeStatus,
    failed_reason: Option<&str>,
) -> CoreResult<()> {
    let mut a = challenges::ActiveModel {
        id: Set(challenge_id.to_string()),
        status: Set(status),
        updated_at: Set(now_rfc3339()),
        ..Default::default()
    };
    if let Some(reason) = failed_reason {
        a.failed_reason = Set(Some(reason.to_string()));
    }
    a.update(&ctx.db).await?;
    emit_challenge(ctx, challenge_id, task_id, domain_id, status);
    Ok(())
}

/// 将本任务全部挑战置失败(CT6,best-effort)并清理 HTTP-01 验证文件。
async fn fail_task_challenges(ctx: &CoreContext, task_id: &str, reason: &str) {
    let Ok(rows) = task_challenges(&ctx.db, task_id).await else {
        return;
    };
    for ch in &rows {
        let _ = update_challenge_status(
            ctx,
            &ch.id,
            task_id,
            &ch.domain_id,
            ChallengeStatus::Failed,
            Some(reason),
        )
        .await;
    }
    cleanup_http_challenge_files(ctx, &ctx.db, &rows).await;
}

/// 清理 HTTP-01 验证文件(best-effort)。挑战行只存 URL 路径(`/.well-known/acme-challenge/<token>`),
/// 据 token + 域名 webroot 重算文件系统路径删除;DNS-01 挑战无文件、跳过。
async fn cleanup_http_challenge_files(
    ctx: &CoreContext,
    db: &DatabaseConnection,
    rows: &[challenges::Model],
) {
    for ch in rows {
        if ch.validation_method != ValidationMethod::Http01 {
            continue;
        }
        let Some(url_path) = ch.http_file_path.as_deref() else {
            continue;
        };
        let Some(token) = url_path.rsplit('/').next().filter(|t| !t.is_empty()) else {
            continue;
        };
        if let Ok(webroot) = resolve_webroot(ctx, db, &ch.domain_id).await {
            let path = webroot.join(".well-known").join("acme-challenge").join(token);
            let _ = std::fs::remove_file(path);
        }
    }
}

/// 发 `challenge_status_changed`(挑战状态机唯一真相在 core,事件仅为失效信号)。
fn emit_challenge(
    ctx: &CoreContext,
    challenge_id: &str,
    task_id: &str,
    domain_id: &str,
    status: ChallengeStatus,
) {
    ctx.emit(DomainEvent::ChallengeStatusChanged {
        challenge_id: challenge_id.to_string(),
        task_id: task_id.to_string(),
        domain_id: domain_id.to_string(),
        status,
    });
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
    ctx.emit(DomainEvent::TaskLogAppended { task_id: task_id.to_string(), seq });
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
