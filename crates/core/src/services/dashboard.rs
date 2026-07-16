//! 仪表盘服务(API dashboard)—— 纯聚合、只读(不落副本)。口径严格基于证书状态(DB1)。

use crate::domain::enums::{CertificateStatus, IssuanceMethod};
use crate::domain::error::CoreResult;
use crate::persistence::entities::{certificate_domains, certificates, domains, tasks};
use crate::services::context::CoreContext;
use sea_orm::*;

pub struct DashboardMetrics {
    pub total_count: u64,
    pub expiring_soon_count: u64,
    pub failed_count: u64,
}

pub struct PendingCertData {
    pub certificate_id: String,
    pub status: CertificateStatus,
    pub domains: Vec<String>,
    pub issuance_method: IssuanceMethod,
    pub not_after: Option<String>,
    pub latest_task_id: Option<String>,
}

pub struct DashboardData {
    pub metrics: DashboardMetrics,
    pub pending_count: u64,
    pub pending_items: Vec<PendingCertData>,
}

const FAILED: [CertificateStatus; 3] = [
    CertificateStatus::IssueFailed,
    CertificateStatus::RenewalFailed,
    CertificateStatus::Expired,
];
const PENDING: [CertificateStatus; 4] = [
    CertificateStatus::Expired,
    CertificateStatus::IssueFailed,
    CertificateStatus::RenewalFailed,
    CertificateStatus::ExpiringSoon,
];

pub async fn overview(ctx: &CoreContext) -> CoreResult<DashboardData> {
    let db = &ctx.db;

    let total_count = certificates::Entity::find().count(db).await?;
    let expiring_soon_count = certificates::Entity::find()
        .filter(certificates::Column::Status.eq(CertificateStatus::ExpiringSoon))
        .count(db)
        .await?;
    let failed_count = certificates::Entity::find()
        .filter(certificates::Column::Status.is_in(FAILED))
        .count(db)
        .await?;

    // 待处理清单:触发集,服务端已按告警级优先(已过期居首)排序
    let mut pending = certificates::Entity::find()
        .filter(certificates::Column::Status.is_in(PENDING))
        .all(db)
        .await?;
    pending.sort_by(|a, b| {
        a.status
            .pending_sort_rank()
            .cmp(&b.status.pending_sort_rank())
            .then_with(|| a.not_after.cmp(&b.not_after))
    });

    let mut pending_items = Vec::with_capacity(pending.len());
    for cert in pending {
        // 关联域名 hostname(经 certificate_domains → domains)
        let link_ids: Vec<String> = certificate_domains::Entity::find()
            .filter(certificate_domains::Column::CertificateId.eq(&cert.id))
            .all(db)
            .await?
            .into_iter()
            .map(|l| l.domain_id)
            .collect();
        let hostnames: Vec<String> = if link_ids.is_empty() {
            vec![]
        } else {
            domains::Entity::find()
                .filter(domains::Column::Id.is_in(link_ids))
                .all(db)
                .await?
                .into_iter()
                .map(|d| d.hostname)
                .collect()
        };
        // 最近一次任务(供跳转查失败原因)
        let latest_task_id = tasks::Entity::find()
            .filter(tasks::Column::CertificateId.eq(&cert.id))
            .order_by_desc(tasks::Column::QueuedAt)
            .one(db)
            .await?
            .map(|t| t.id);

        pending_items.push(PendingCertData {
            certificate_id: cert.id,
            status: cert.status,
            domains: hostnames,
            issuance_method: cert.issuance_method,
            not_after: cert.not_after,
            latest_task_id,
        });
    }

    Ok(DashboardData {
        metrics: DashboardMetrics { total_count, expiring_soon_count, failed_count },
        pending_count: expiring_soon_count + failed_count,
        pending_items,
    })
}
