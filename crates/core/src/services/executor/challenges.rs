//! ACME 验证挑战的准备、持久化和清理。

use super::*;
use crate::domain::enums::{ChallengeStatus, ValidationMethod};
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{
    acme_accounts, certificate_domains, certificates, challenges, domains,
    http01_validation_configs, tasks,
};
use crate::services::acme as acme_service;
use crate::util::{new_id, now_rfc3339};
use instant_acme::{ChallengeType, Order};
use std::path::{Path, PathBuf};

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
    let acme_account = acme_service::load_acme_account(ctx, &account_row).await?;
    let mut order = acme_account
        .order(order_url)
        .await
        .map_err(acme_service::map_acme_err)?;
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
    log(
        ctx,
        &task.id,
        "info",
        &format!("域名 {hostname}:已确认 TXT,通知 CA 校验"),
    )
    .await?;

    // 本任务是否仍有等待手动/待验证的挑战?有则等其余确认;无则续推 finalize。
    let pending = task_challenges(db, &task.id)
        .await?
        .into_iter()
        .filter(|c| {
            matches!(
                c.status,
                ChallengeStatus::AwaitingManual | ChallengeStatus::Pending
            )
        })
        .count();
    if pending > 0 {
        dashboard::emit_changed(ctx).await;
        return Ok(());
    }

    // 全部就绪 → 续推(仅当证书仍处进行中态,防重复 finalize)。
    if !matches!(
        cert.status,
        CertificateStatus::Issuing | CertificateStatus::Renewing
    ) {
        dashboard::emit_changed(ctx).await;
        return Ok(());
    }
    match super::acme::finalize_acme_order(ctx, &task, &cert, &mut order).await {
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
        return Err(CoreError::new(
            ErrorCode::ChallengeNotRetryable,
            "仅失败的挑战可重试",
        ));
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

pub(super) async fn challenge_task_context(
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
    let account_row = super::acme::load_registered_account(ctx, &cert).await?;
    Ok((task, cert, account_row))
}

/// 在重建的订单中定位某域名授权对应类型的挑战并 `set_ready`(通知 CA 校验)。
pub(super) async fn set_ready_for_domain(
    order: &mut Order,
    hostname: &str,
    r#type: ChallengeType,
) -> CoreResult<()> {
    let mut authorizations = order.authorizations();
    while let Some(result) = authorizations.next().await {
        let mut authz = result.map_err(acme_service::map_acme_err)?;
        if authz.identifier().to_string() != hostname {
            continue;
        }
        let mut challenge = authz
            .challenge(r#type.clone())
            .ok_or_else(|| CoreError::internal(format!("域名 {hostname} 无对应挑战")))?;
        challenge
            .set_ready()
            .await
            .map_err(acme_service::map_acme_err)?;
        return Ok(());
    }
    Err(CoreError::internal(format!(
        "订单中未找到域名 {hostname} 的授权"
    )))
}

/// DNS-01 待添加 TXT 记录名:`_acme-challenge.<base>`;通配符 `*.example.com` 取 `example.com`。
pub(super) fn dns_txt_name(hostname: &str) -> String {
    let base = hostname.strip_prefix("*.").unwrap_or(hostname);
    format!("_acme-challenge.{base}")
}

/// 证书 SAN 域名行(acme 需 hostname + 验证方式)。
pub(super) async fn acme_san_domains(
    db: &DatabaseConnection,
    cert_id: &str,
) -> CoreResult<Vec<domains::Model>> {
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
        .await?)
}

/// 解析域名 HTTP-01 webroot:有配置用之;无则用数据目录下临时 webroot(DEA5 / 本切片兜底)。
pub(super) async fn resolve_webroot(
    ctx: &CoreContext,
    db: &DatabaseConnection,
    domain_id: &str,
) -> CoreResult<PathBuf> {
    if let Some(cfg) = http01_validation_configs::Entity::find_by_id(domain_id)
        .one(db)
        .await?
    {
        Ok(PathBuf::from(cfg.webroot_path))
    } else {
        Ok(ctx.data_dir.join("acme-webroot").join(domain_id))
    }
}

/// 在 webroot 下写 `.well-known/acme-challenge/<token>` 验证文件,返回其文件系统路径。
pub(super) async fn write_challenge_file(
    webroot: &Path,
    token: &str,
    key_auth: &str,
) -> CoreResult<PathBuf> {
    let dir = webroot.join(".well-known").join("acme-challenge");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| CoreError::internal(format!("创建 webroot 目录失败: {e}")))?;
    let path = dir.join(token);
    tokio::fs::write(&path, key_auth.as_bytes())
        .await
        .map_err(|e| CoreError::internal(format!("写入验证文件失败: {e}")))?;
    Ok(path)
}

/// 挑战记录的可选展示/执行字段(HTTP-01 文件 / DNS-01 TXT / 订单引用)。
#[derive(Default)]
pub(super) struct ChallengeFields {
    pub(super) dns_txt_name: Option<String>,
    pub(super) dns_txt_value: Option<String>,
    pub(super) http_file_path: Option<String>,
    pub(super) http_file_content: Option<String>,
    pub(super) authorization_url: Option<String>,
}

/// 建挑战记录(CT1,pending)+ 发 `challenge_status_changed`。返回挑战 id。验证方式与字段由调用方给出
/// (HTTP-01 传文件路径/内容;DNS-01 传 TXT 名/值);`authorization_url` 存 order URL 供 confirm/retry 重建订单。
pub(super) async fn insert_challenge(
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
pub(super) async fn task_challenges(
    db: &DatabaseConnection,
    task_id: &str,
) -> CoreResult<Vec<challenges::Model>> {
    Ok(challenges::Entity::find()
        .filter(challenges::Column::TaskId.eq(task_id))
        .all(db)
        .await?)
}

/// 推进挑战状态(+ 可选失败原因)并发 `challenge_status_changed`。
pub(super) async fn update_challenge_status(
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
pub(super) async fn fail_task_challenges(ctx: &CoreContext, task_id: &str, reason: &str) {
    let rows = match task_challenges(&ctx.db, task_id).await {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(task_id, error = %error, "读取 ACME 挑战失败,跳过失败态清理");
            return;
        }
    };
    for ch in &rows {
        if let Err(error) = update_challenge_status(
            ctx,
            &ch.id,
            task_id,
            &ch.domain_id,
            ChallengeStatus::Failed,
            Some(reason),
        )
        .await
        {
            tracing::warn!(challenge_id = %ch.id, error = %error, "更新 ACME 挑战失败态失败");
        }
    }
    cleanup_http_challenge_files(ctx, &ctx.db, &rows).await;
}

/// 清理 HTTP-01 验证文件(best-effort)。挑战行只存 URL 路径(`/.well-known/acme-challenge/<token>`),
/// 据 token + 域名 webroot 重算文件系统路径删除;DNS-01 挑战无文件、跳过。
pub(super) async fn cleanup_http_challenge_files(
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
        let webroot = match resolve_webroot(ctx, db, &ch.domain_id).await {
            Ok(webroot) => webroot,
            Err(error) => {
                tracing::warn!(domain_id = %ch.domain_id, error = %error, "解析 HTTP-01 webroot 失败,保留验证文件");
                continue;
            }
        };
        let path = webroot
            .join(".well-known")
            .join("acme-challenge")
            .join(token);
        if let Err(error) = tokio::fs::remove_file(&path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %path.display(), error = %error, "清理 HTTP-01 验证文件失败");
            }
        }
    }
}

/// 发 `challenge_status_changed`(挑战状态机唯一真相在 core,事件仅为失效信号)。
pub(super) fn emit_challenge(
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
