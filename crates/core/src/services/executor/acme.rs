//! ACME 订单、签发/续签和吊销执行流程。

use super::challenges::*;
use super::*;
use crate::ca;
use crate::domain::enums::{AcmeAccountStatus, ChallengeStatus, ValidationMethod};
use crate::persistence::entities::acme_accounts;
use crate::services::acme as acme_service;
use instant_acme::{
    AuthorizationStatus, ChallengeStatus as AcmeChallengeStatus, ChallengeType, Identifier,
    NewOrder, Order, OrderStatus, RetryPolicy, RevocationRequest,
};
use rustls_pki_types::CertificateDer;

/// acme 签发/续签(同一 order 流程,首签/续签不区分 acme DA5)。issue:T2 issuing→T3 valid;
/// renew:T12 renewing→valid(刷新同一行 serial/有效期,不新建实体 DC1)。经 instant-acme:建单 →
/// 每域名取挑战(HTTP-01 放 webroot 文件、通知就绪 CT2;DNS-01 计算 TXT 置 `awaiting_manual` CT3
/// **挂起等待手动**)→ 全部就绪则 finalize 取证;任一域名 DNS-01 挂起则 `ExecOutcome::Suspended`
/// (任务让出 worker,confirm 续推)。任一失败整体失败交回 issue_failed/renewal_failed(§3.4)。
pub(super) async fn run_acme_issue_or_renew(
    ctx: &CoreContext,
    task: &tasks::Model,
    cert: &certificates::Model,
) -> CoreResult<ExecOutcome> {
    let db = &ctx.db;
    let is_renew = matches!(task.task_type, TaskType::Renew);

    // T2/T12:进行中态(issue→issuing / renew→renewing;renew 幂等确认,兼容重试链直达)
    let in_progress = if is_renew {
        CertificateStatus::Renewing
    } else {
        CertificateStatus::Issuing
    };
    update_cert_status(ctx, &cert.id, in_progress).await?;
    log(
        ctx,
        &task.id,
        "info",
        if is_renew {
            "证书置为续签中(ACME)"
        } else {
            "证书置为签发中(ACME)"
        },
    )
    .await?;

    // ACME 账户(须 registered、有账户密钥)
    let account_row = load_registered_account(ctx, cert).await?;
    log(
        ctx,
        &task.id,
        "info",
        &format!("使用 ACME 账户「{}」", account_row.contact_email),
    )
    .await?;

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
    log(
        ctx,
        &task.id,
        "info",
        &format!("SAN: {}", hostnames.join(", ")),
    )
    .await?;

    // 载入账户 + 建订单(SAN 域名)。捕获 order URL 存挑战 `authorization_url`,供 confirm/retry 重建订单。
    let acme_account = acme_service::load_acme_account(ctx, &account_row).await?;
    let identifiers: Vec<Identifier> = hostnames
        .iter()
        .map(|h| Identifier::Dns(h.clone()))
        .collect();
    let mut order = acme_account
        .new_order(&NewOrder::new(&identifiers))
        .await
        .map_err(acme_service::map_acme_err)?;
    let order_url = order.url().to_string();
    log(ctx, &task.id, "info", "已向 CA 建立订单").await?;

    // 每域名:建挑战记录(pending CT1)→ HTTP-01 放文件+通知就绪(validating CT2)/ DNS-01 计算 TXT
    // 置 awaiting_manual(CT3,挂起)。任一域名进入 awaiting_manual → 本次整体挂起。
    let mut any_awaiting = false;
    {
        let mut authorizations = order.authorizations();
        while let Some(result) = authorizations.next().await {
            let mut authz = result.map_err(acme_service::map_acme_err)?;
            let identifier = authz.identifier().to_string();
            let domain = san
                .iter()
                .find(|d| d.hostname == identifier)
                .ok_or_else(|| {
                    CoreError::internal(format!("授权域名 {identifier} 不在证书 SAN 中"))
                })?;

            match domain.validation_method {
                // CA 复用仍在有效期内的授权(签发缓存,LE ~30 天):不下发任何挑战,
                // 该域名直接视为已验证 —— 记一条 passed 挑战行(审计/展示),不再取挑战。
                _ if authz.status == AuthorizationStatus::Valid => {
                    let challenge_id = insert_challenge(
                        ctx,
                        &task.id,
                        &domain.id,
                        domain.validation_method.ok_or_else(|| {
                            CoreError::new(
                                ErrorCode::DomainValidationMethodRequired,
                                format!("域名 {} 未设置验证方式", domain.hostname),
                            )
                        })?,
                        ChallengeFields {
                            authorization_url: Some(order_url.clone()),
                            ..Default::default()
                        },
                    )
                    .await?;
                    update_challenge_status(
                        ctx,
                        &challenge_id,
                        &task.id,
                        &domain.id,
                        ChallengeStatus::Passed,
                        None,
                    )
                    .await?;
                    log(
                        ctx,
                        &task.id,
                        "info",
                        &format!("域名 {identifier}:复用仍在有效期内的授权,无需重复验证"),
                    )
                    .await?;
                }
                Some(ValidationMethod::Http01) => {
                    let Some(mut challenge) = authz.challenge(ChallengeType::Http01) else {
                        return Err(CoreError::internal(format!(
                            "域名 {identifier} 无 HTTP-01 挑战"
                        )));
                    };
                    let token = challenge.token.clone();
                    let key_auth = challenge.key_authorization().as_str().to_string();
                    let needs_ready = matches!(challenge.status, AcmeChallengeStatus::Pending);

                    // 放验证文件(always-valid 下可达性不重要,仍走流程);清理由 finalize 依 token 重算路径。
                    let webroot = resolve_webroot(ctx, db, &domain.id).await?;
                    write_challenge_file(&webroot, &token, &key_auth).await?;
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
                    log(
                        ctx,
                        &task.id,
                        "info",
                        &format!("域名 {identifier}:HTTP-01 验证文件已放置"),
                    )
                    .await?;

                    if needs_ready {
                        challenge
                            .set_ready()
                            .await
                            .map_err(acme_service::map_acme_err)?;
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
                        return Err(CoreError::internal(format!(
                            "域名 {identifier} 无 DNS-01 挑战"
                        )));
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
                        &format!(
                            "域名 {identifier}:DNS-01 待手动添加 TXT 记录 {txt_name},等待确认"
                        ),
                    )
                    .await?;
                    any_awaiting = true;
                }
                None => {
                    return Err(CoreError::new(
                        ErrorCode::DomainValidationMethodRequired,
                        format!("域名 {} 未设置验证方式", domain.hostname),
                    ));
                }
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
pub(super) async fn finalize_acme_order(
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
            let err = acme_service::map_acme_err(e);
            fail_task_challenges(ctx, &task.id, "订单验证失败").await;
            return Err(err);
        }
    };
    if ready != OrderStatus::Ready {
        fail_task_challenges(ctx, &task.id, "挑战未通过").await;
        return Err(CoreError::internal(format!(
            "订单验证未通过(状态 {ready:?})"
        )));
    }

    // 全部域名验证通过(CT5,passed)
    let ch_rows = task_challenges(db, &task.id).await?;
    for ch in &ch_rows {
        update_challenge_status(
            ctx,
            &ch.id,
            &task.id,
            &ch.domain_id,
            ChallengeStatus::Passed,
            None,
        )
        .await?;
    }
    log(ctx, &task.id, "info", "全部域名验证通过").await?;

    // finalize(返回叶子私钥 PEM)→ 取证(链 PEM:叶子+中间)
    let leaf_key_pem = match order.finalize().await {
        Ok(k) => k,
        Err(e) => {
            let err = acme_service::map_acme_err(e);
            fail_task_challenges(ctx, &task.id, "取证失败").await;
            return Err(err);
        }
    };
    let chain_pem = match order.poll_certificate(&poll).await {
        Ok(c) => c,
        Err(e) => {
            let err = acme_service::map_acme_err(e);
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
        &format!(
            "叶子证书序列号 {},有效期至 {}",
            meta.serial_number, meta.not_after
        ),
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

    let verb = if matches!(task.task_type, TaskType::Renew) {
        "续签"
    } else {
        "签发"
    };
    Ok(format!("{verb}成功(序列号 {})", meta.serial_number))
}

/// acme 吊销(T18 revoking → revoked)。向 CA 发吊销请求(revoke_cert):取证书链叶子 DER →
/// `Account::revoke`。成功后证书 `revoking→revoked`。失败上层回退证书(T19,drive_cert_failure)。
pub(super) async fn run_revoke_acme(
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
    let acme_account = acme_service::load_acme_account(ctx, &account_row).await?;
    let certificate = CertificateDer::from(leaf_der);
    let req = RevocationRequest {
        certificate: &certificate,
        reason: None,
    };
    acme_account
        .revoke(&req)
        .await
        .map_err(acme_service::map_acme_err)?;
    log(ctx, &task.id, "info", "已向 CA 提交吊销请求").await?;

    // T18:revoking → revoked
    update_cert_status(ctx, &cert.id, CertificateStatus::Revoked).await?;

    let serial = cert.serial_number.clone().unwrap_or_default();
    Ok(format!("已吊销(序列号 {serial})"))
}

/// 载入证书关联的 ACME 账户行,校验为 `registered`(issue/renew/revoke 共用前置)。
pub(super) async fn load_registered_account(
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
