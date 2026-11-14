//! self_signed 签发、续签和吊销执行流程。

use super::*;
use crate::ca;
use crate::domain::enums::RootCaStatus;
use crate::persistence::entities::{internal_cert_revocations, root_cas};

/// self_signed 签发(T2 issuing → T3 valid):用指定根 CA 签发叶子(SAN=证书关联域名)。
pub(super) async fn run_issue_self_signed(
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
        return Err(CoreError::new(
            ErrorCode::RootCaExpired,
            "根 CA 已过期,拒绝签发",
        ));
    }
    log(
        ctx,
        &task.id,
        "info",
        &format!("使用根 CA「{}」签发", root_ca.name),
    )
    .await?;

    // SAN 域名
    let hostnames = san_hostnames(db, &cert.id).await?;
    if hostnames.is_empty() {
        return Err(CoreError::internal("证书无关联域名(SAN 为空),无法签发"));
    }
    log(
        ctx,
        &task.id,
        "info",
        &format!("SAN: {}", hostnames.join(", ")),
    )
    .await?;

    // 读根 CA 私钥(age 解密)→ 签发叶子(私钥材料仅在内存,不入日志)
    let root_key_pem = String::from_utf8(ctx.secrets.load(&root_ca.private_key_ref)?)
        .map_err(|_| CoreError::internal("根 CA 私钥材料损坏"))?;
    let leaf = ca::sign_leaf(
        &root_ca.cert_pem,
        &root_key_pem,
        &hostnames,
        LEAF_VALIDITY_DAYS,
    )?;
    log(
        ctx,
        &task.id,
        "info",
        &format!(
            "叶子证书已签发,序列号 {},有效期至 {}",
            leaf.serial_number, leaf.not_after
        ),
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
pub(super) async fn run_renew_self_signed(
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
        return Err(CoreError::new(
            ErrorCode::RootCaExpired,
            "根 CA 已过期,拒绝续签",
        ));
    }
    log(
        ctx,
        &task.id,
        "info",
        &format!("经原根 CA「{}」重签", root_ca.name),
    )
    .await?;

    let hostnames = san_hostnames(db, &cert.id).await?;
    if hostnames.is_empty() {
        return Err(CoreError::internal("证书无关联域名(SAN 为空),无法续签"));
    }

    // 重签叶子(rcgen 每次生成新密钥 → 换新私钥,呼应 T20 语义)
    let root_key_pem = String::from_utf8(ctx.secrets.load(&root_ca.private_key_ref)?)
        .map_err(|_| CoreError::internal("根 CA 私钥材料损坏"))?;
    let leaf = ca::sign_leaf(
        &root_ca.cert_pem,
        &root_key_pem,
        &hostnames,
        LEAF_VALIDITY_DAYS,
    )?;
    log(
        ctx,
        &task.id,
        "info",
        &format!(
            "续签完成,新序列号 {},有效期至 {}",
            leaf.serial_number, leaf.not_after
        ),
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
pub(super) async fn run_revoke_self_signed(
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
    log(
        ctx,
        &task.id,
        "info",
        &format!("已在根 CA 记录作废,序列号 {serial}"),
    )
    .await?;

    // T18:revoking → revoked
    update_cert_status(ctx, &cert.id, CertificateStatus::Revoked).await?;

    Ok(format!("已吊销(序列号 {serial})"))
}
