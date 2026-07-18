//! 扫描调度器(certificates 拥有,ARCHITECTURE §6.2 / §7;certificates flow §1.1)。
//!
//! 一个 tokio 周期任务 + **boot 启动即全量扫描**(§7 step2/3),驱动到期状态机流转并按 settings
//! 触发自动续签:
//! - 证书 `valid → expiring_soon`(T6,距 `not_after` ≤ settings `renewalAdvanceDays`)、
//!   `expiring_soon → expired`(T10,过 `not_after`);
//! - 根 CA `active → expired`(local-ca L3,过有效期);
//! - `autoRenewEnabled` 开:对 `expiring_soon` 发起续签任务(auto 触发,T9)、对 `renewal_failed`
//!   且**未过期**者随扫描周期再尝试(依附扫描,无独立重试参数,SF2)。续签由执行器承接:self_signed
//!   直接重签;acme HTTP-01 自动完成、DNS-01 挂起于 `awaiting_manual` 等用户 confirm(见 executor)。
//!
//! 扫描周期为实现机制、**不暴露配置**(settings SF3);合理默认见 [`SCAN_INTERVAL`]。

use crate::domain::enums::{CertificateStatus, RootCaStatus, TaskStatus};
use crate::domain::error::CoreResult;
use crate::domain::events::DomainEvent;
use crate::persistence::entities::{certificates, root_cas, tasks};
use crate::services::context::CoreContext;
use crate::services::{certificates as cert_svc, dashboard, settings as settings_svc};
use crate::util::{days_until, now_rfc3339};
use sea_orm::*;
use std::time::Duration;

/// 扫描周期(实现机制,不可配 SF3)。到期判定 + 自动续签检测频率;默认 60s 兼顾时效与开销。
const SCAN_INTERVAL: Duration = Duration::from_secs(60);

/// 单次扫描的推进统计(供日志/自验证)。
#[derive(Debug, Default, Clone, Copy)]
pub struct ScanReport {
    pub certs_expiring_soon: u64,
    pub certs_expired: u64,
    pub root_cas_expired: u64,
    pub auto_renews_started: u64,
}

impl ScanReport {
    fn changed(&self) -> bool {
        self.certs_expiring_soon
            + self.certs_expired
            + self.root_cas_expired
            + self.auto_renews_started
            > 0
    }
}

/// 启动周期扫描循环(boot 之后由 bin 调用)。首扫已在 `boot::run` 完成,这里只跑周期。
pub fn spawn(ctx: CoreContext) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!(
            interval_secs = SCAN_INTERVAL.as_secs(),
            "证书/根CA 扫描调度器已启动"
        );
        loop {
            tokio::time::sleep(SCAN_INTERVAL).await;
            match scan_once(&ctx).await {
                Ok(r) if r.changed() => tracing::info!(?r, "扫描:状态推进"),
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "扫描 tick 失败,稍后重试"),
            }
        }
    })
}

/// 单次全量扫描:到期判定 + 自动续签。boot 启动即调用一次(§7),此后周期调用。
pub async fn scan_once(ctx: &CoreContext) -> CoreResult<ScanReport> {
    let db = &ctx.db;
    let mut report = ScanReport::default();

    // settings:续签提前天数 + 自动续签开关(get_or_init 幂等 upsert 默认行)。
    let settings = settings_svc::get_or_init(ctx).await?;
    let advance_days = settings.renewal_advance_days as i64;
    let auto_renew = settings.auto_renew_enabled;

    // 1) 证书到期判定(T6 / T10)。仅活跃到期相关态:valid / expiring_soon。
    let certs = certificates::Entity::find()
        .filter(
            certificates::Column::Status
                .is_in([CertificateStatus::Valid, CertificateStatus::ExpiringSoon]),
        )
        .all(db)
        .await?;
    for cert in certs {
        let days = days_until(cert.not_after.as_deref());
        let mut status = cert.status;
        // T6:valid → expiring_soon(距到期 ≤ 提前天数;含已越过到期的 valid,先入 expiring_soon,
        // 同轮下方再判 expired,单次扫描即落到应有终态,保首屏一致 §7)。
        if status == CertificateStatus::Valid {
            if let Some(d) = days {
                if d <= advance_days {
                    set_cert_status(ctx, &cert.id, CertificateStatus::ExpiringSoon).await?;
                    status = CertificateStatus::ExpiringSoon;
                    report.certs_expiring_soon += 1;
                }
            }
        }
        // T10:expiring_soon → expired(已过 not_after)。
        if status == CertificateStatus::ExpiringSoon {
            if let Some(d) = days {
                if d < 0 {
                    set_cert_status(ctx, &cert.id, CertificateStatus::Expired).await?;
                    report.certs_expired += 1;
                }
            }
        }
    }

    // 2) 根 CA 到期(L3:active → expired)。
    let active_cas = root_cas::Entity::find()
        .filter(root_cas::Column::Status.eq(RootCaStatus::Active))
        .all(db)
        .await?;
    for ca in active_cas {
        if days_until(Some(ca.not_after.as_str())).is_some_and(|d| d < 0) {
            let mut a: root_cas::ActiveModel = ca.clone().into();
            a.status = Set(RootCaStatus::Expired);
            a.updated_at = Set(now_rfc3339());
            a.update(db).await?;
            ctx.emit(DomainEvent::RootCaStatusChanged {
                root_ca_id: ca.id.clone(),
                status: RootCaStatus::Expired,
            });
            report.root_cas_expired += 1;
        }
    }

    // 3) 自动续签(settings 开关)。expiring_soon → 续签(T9);renewal_failed 且未过期 → 再尝试(SF2)。
    if auto_renew {
        let candidates = certificates::Entity::find()
            .filter(certificates::Column::Status.is_in([
                CertificateStatus::ExpiringSoon,
                CertificateStatus::RenewalFailed,
            ]))
            .all(db)
            .await?;
        for cert in candidates {
            // renewal_failed 仅"未过期"者再尝试(SF2);已过期则留于 renewal_failed(失败桶),不再空转。
            if cert.status == CertificateStatus::RenewalFailed
                && days_until(cert.not_after.as_deref()).is_some_and(|d| d < 0)
            {
                continue;
            }
            // 已有进行中任务(queued/running)则跳过,避免对同一证书重复入队。
            if has_active_task(db, &cert.id).await? {
                continue;
            }
            // 来源前置由 cert_svc::auto_renew 校验;不满足(根 CA 过期 / 账户未注册)则跳过,避免失败循环。
            if cert_svc::auto_renew(ctx, &cert).await? {
                report.auto_renews_started += 1;
            }
        }
    }

    // 待处理集合可能变动 → 发红点合并信号(桌面托盘角标 / dashboard 聚合)。
    if report.changed() {
        dashboard::emit_changed(ctx).await;
    }
    Ok(report)
}

/// 更新证书状态 + updated_at,并发 `certificate_status_changed` 事件。
async fn set_cert_status(ctx: &CoreContext, id: &str, status: CertificateStatus) -> CoreResult<()> {
    certificates::ActiveModel {
        id: Set(id.to_string()),
        status: Set(status),
        updated_at: Set(now_rfc3339()),
        ..Default::default()
    }
    .update(&ctx.db)
    .await?;
    ctx.emit(DomainEvent::CertificateStatusChanged {
        certificate_id: id.to_string(),
        status,
    });
    Ok(())
}

/// 该证书是否有进行中任务(queued/running)。
async fn has_active_task(db: &DatabaseConnection, cert_id: &str) -> CoreResult<bool> {
    let n = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(cert_id))
        .filter(tasks::Column::Status.is_in([TaskStatus::Queued, TaskStatus::Running]))
        .count(db)
        .await?;
    Ok(n > 0)
}
