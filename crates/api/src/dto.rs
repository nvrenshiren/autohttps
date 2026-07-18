//! DTO(camelCase,ts-rs 可派生)—— 传输契约表面(common §1)。
//!
//! **密钥边界**(AR4 / common §7):DTO 绝不含 `private_key_ref` / `account_key_ref` / `cert_pem_ref`
//! 或任何密钥材料。`daysUntilExpiry` 等计算字段相对服务器 now 算出(common §1)。
//!
//! DTO 是 Rust 真相的投影;前端 `frontend/src/bindings/` 为其 TS 映射(手写提交,与本文件同步)。

#![allow(clippy::too_many_arguments)]

use autohttps_core::enums::*;
use autohttps_core::services::pagination::Paged;
use autohttps_core::services::{acme, certificates, dashboard, domains, local_ca, tasks};
use autohttps_core::util::days_until;
use serde::Serialize;
use ts_rs::TS;

// ============ 通用 ============

/// 分页响应包络(TECH §3.3,定死)。
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Page<T: TS> {
    pub items: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

/// 由 core 的 `Paged<S>` + 映射函数构造响应包络。
pub fn page_of<S, T: TS>(p: Paged<S>, f: impl Fn(S) -> T) -> Page<T> {
    Page {
        items: p.items.into_iter().map(f).collect(),
        page: p.page,
        page_size: p.page_size,
        total: p.total,
    }
}

/// 运行形态标志(common §6.2)。
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub run_mode: RunMode,
    pub app_version: String,
}

// ============ certificates ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DomainRef {
    pub id: String,
    pub hostname: String,
    pub is_wildcard: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CertificateSummary {
    pub id: String,
    pub status: CertificateStatus,
    pub issuance_method: IssuanceMethod,
    pub domains: Vec<DomainRef>,
    pub serial_number: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub days_until_expiry: Option<i64>,
    pub is_exportable: bool,
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AcmeAccountRef {
    pub id: String,
    pub ca_label: Option<String>,
    pub environment: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootCaRef {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CertificateDetail {
    pub id: String,
    pub status: CertificateStatus,
    pub issuance_method: IssuanceMethod,
    pub domains: Vec<DomainRef>,
    pub serial_number: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub days_until_expiry: Option<i64>,
    pub is_exportable: bool,
    pub last_error: Option<String>,
    pub updated_at: String,
    pub fingerprint: Option<String>,
    pub issued_at: Option<String>,
    pub created_at: String,
    pub acme_account: Option<AcmeAccountRef>,
    pub root_ca: Option<RootCaRef>,
    pub active_task_id: Option<String>,
}

fn domain_refs(ds: Vec<certificates::DomainRefData>) -> Vec<DomainRef> {
    ds.into_iter()
        .map(|d| DomainRef {
            id: d.id,
            hostname: d.hostname,
            is_wildcard: d.is_wildcard,
        })
        .collect()
}

pub fn cert_summary(row: certificates::CertRow) -> CertificateSummary {
    let c = row.cert;
    CertificateSummary {
        days_until_expiry: days_until(c.not_after.as_deref()),
        is_exportable: c.status.is_exportable(),
        id: c.id,
        status: c.status,
        issuance_method: c.issuance_method,
        domains: domain_refs(row.domains),
        serial_number: c.serial_number,
        not_before: c.not_before,
        not_after: c.not_after,
        last_error: c.last_error,
        updated_at: c.updated_at,
    }
}

pub fn cert_detail(data: certificates::CertDetailData) -> CertificateDetail {
    let c = data.row.cert;
    CertificateDetail {
        days_until_expiry: days_until(c.not_after.as_deref()),
        is_exportable: c.status.is_exportable(),
        id: c.id,
        status: c.status,
        issuance_method: c.issuance_method,
        domains: domain_refs(data.row.domains),
        serial_number: c.serial_number,
        not_before: c.not_before,
        not_after: c.not_after,
        last_error: c.last_error,
        updated_at: c.updated_at,
        fingerprint: c.fingerprint,
        issued_at: c.issued_at,
        created_at: c.created_at,
        acme_account: data.acme_account.map(|a| AcmeAccountRef {
            id: a.id,
            ca_label: a.ca_label,
            environment: a.environment,
        }),
        root_ca: data.root_ca.map(|r| RootCaRef {
            id: r.id,
            name: r.name,
        }),
        active_task_id: data.active_task_id,
    }
}

// ============ domains ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DomainSummary {
    pub id: String,
    pub hostname: String,
    pub is_wildcard: bool,
    pub group_name: Option<String>,
    pub remark: Option<String>,
    pub validation_method: Option<ValidationMethod>,
    pub certificate_count: u64,
    pub worst_certificate_status: Option<CertificateStatus>,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DomainCertificateRef {
    pub id: String,
    pub status: CertificateStatus,
    pub issuance_method: IssuanceMethod,
    pub not_after: Option<String>,
    pub days_until_expiry: Option<i64>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DomainDetail {
    pub id: String,
    pub hostname: String,
    pub is_wildcard: bool,
    pub group_name: Option<String>,
    pub remark: Option<String>,
    pub validation_method: Option<ValidationMethod>,
    pub certificate_count: u64,
    pub worst_certificate_status: Option<CertificateStatus>,
    pub updated_at: String,
    pub created_at: String,
    pub certificates: Vec<DomainCertificateRef>,
}

pub fn domain_summary(row: domains::DomainRow) -> DomainSummary {
    let d = row.domain;
    DomainSummary {
        id: d.id,
        hostname: d.hostname,
        is_wildcard: d.is_wildcard,
        group_name: d.group_name,
        remark: d.remark,
        validation_method: d.validation_method,
        certificate_count: row.certificate_count,
        worst_certificate_status: row.worst_status,
        updated_at: d.updated_at,
    }
}

pub fn domain_detail(data: domains::DomainDetailData) -> DomainDetail {
    let d = data.row.domain;
    DomainDetail {
        id: d.id,
        hostname: d.hostname,
        is_wildcard: d.is_wildcard,
        group_name: d.group_name,
        remark: d.remark,
        validation_method: d.validation_method,
        certificate_count: data.row.certificate_count,
        worst_certificate_status: data.row.worst_status,
        updated_at: d.updated_at,
        created_at: d.created_at,
        certificates: data
            .certificates
            .into_iter()
            .map(|p| DomainCertificateRef {
                days_until_expiry: days_until(p.not_after.as_deref()),
                id: p.id,
                status: p.status,
                issuance_method: p.issuance_method,
                not_after: p.not_after,
            })
            .collect(),
    }
}

// ============ settings ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub renewal_advance_days: i32,
    pub auto_renew_enabled: bool,
    pub default_acme_account_id: Option<String>,
    /// 仅桌面;服务器形态为 null。
    pub autostart_enabled: Option<bool>,
    /// 仅服务器;桌面形态为 null。
    pub listen_address: Option<String>,
    pub listen_port: Option<i32>,
    /// 只读展示(SF5)。
    pub data_storage_path: String,
    pub updated_at: String,
}

pub fn settings_view(
    m: autohttps_core::persistence::entities::settings::Model,
    run_mode: RunMode,
) -> SettingsView {
    let is_desktop = matches!(run_mode, RunMode::Desktop);
    SettingsView {
        renewal_advance_days: m.renewal_advance_days,
        auto_renew_enabled: m.auto_renew_enabled,
        default_acme_account_id: m.default_acme_account_id,
        // 按当前形态取其一有值、另一组 null(database 形态差异)
        autostart_enabled: if is_desktop {
            Some(m.autostart_enabled.unwrap_or(false))
        } else {
            None
        },
        listen_address: if is_desktop { None } else { m.listen_address },
        listen_port: if is_desktop { None } else { m.listen_port },
        data_storage_path: m.data_storage_path,
        updated_at: m.updated_at,
    }
}

// ============ dashboard ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub total_count: u64,
    pub expiring_soon_count: u64,
    pub failed_count: u64,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PendingCertItem {
    pub certificate_id: String,
    pub status: CertificateStatus,
    pub domains: Vec<String>,
    pub issuance_method: IssuanceMethod,
    pub not_after: Option<String>,
    pub days_until_expiry: Option<i64>,
    pub latest_task_id: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DashboardOverview {
    pub metrics: DashboardMetrics,
    pub pending_count: u64,
    pub pending_items: Vec<PendingCertItem>,
}

pub fn dashboard_overview(data: dashboard::DashboardData) -> DashboardOverview {
    DashboardOverview {
        metrics: DashboardMetrics {
            total_count: data.metrics.total_count,
            expiring_soon_count: data.metrics.expiring_soon_count,
            failed_count: data.metrics.failed_count,
        },
        pending_count: data.pending_count,
        pending_items: data
            .pending_items
            .into_iter()
            .map(|p| PendingCertItem {
                days_until_expiry: days_until(p.not_after.as_deref()),
                certificate_id: p.certificate_id,
                status: p.status,
                domains: p.domains,
                issuance_method: p.issuance_method,
                not_after: p.not_after,
                latest_task_id: p.latest_task_id,
            })
            .collect(),
    }
}

// ============ tasks ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: String,
    pub certificate_id: String,
    pub certificate_deleted: bool,
    pub certificate_domains: Option<Vec<String>>,
    pub task_type: TaskType,
    pub trigger: TaskTrigger,
    pub status: TaskStatus,
    pub attempt_number: i32,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskCertificateRef {
    pub id: String,
    pub status: CertificateStatus,
    pub domains: Vec<String>,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub id: String,
    pub certificate_id: String,
    pub certificate_deleted: bool,
    pub certificate_domains: Option<Vec<String>>,
    pub task_type: TaskType,
    pub trigger: TaskTrigger,
    pub status: TaskStatus,
    pub attempt_number: i32,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
    pub parent_task_id: Option<String>,
    pub child_task_ids: Vec<String>,
    pub certificate: Option<TaskCertificateRef>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogEntry {
    pub id: String,
    pub task_id: String,
    pub seq: i32,
    pub logged_at: String,
    pub level: String,
    pub message: String,
}

pub fn task_summary(row: tasks::TaskRow) -> TaskSummary {
    let t = row.task;
    TaskSummary {
        id: t.id,
        certificate_id: t.certificate_id,
        certificate_deleted: row.certificate_deleted,
        certificate_domains: row.certificate_domains,
        task_type: t.task_type,
        trigger: t.trigger,
        status: t.status,
        attempt_number: t.attempt_number,
        queued_at: t.queued_at,
        started_at: t.started_at,
        finished_at: t.finished_at,
        result_summary: t.result_summary,
        failure_reason: t.failure_reason,
    }
}

pub fn task_detail(data: tasks::TaskDetailData) -> TaskDetail {
    let t = data.row.task;
    TaskDetail {
        id: t.id,
        certificate_id: t.certificate_id,
        certificate_deleted: data.row.certificate_deleted,
        certificate_domains: data.row.certificate_domains,
        task_type: t.task_type,
        trigger: t.trigger,
        status: t.status,
        attempt_number: t.attempt_number,
        queued_at: t.queued_at,
        started_at: t.started_at,
        finished_at: t.finished_at,
        result_summary: t.result_summary,
        failure_reason: t.failure_reason,
        parent_task_id: data.parent_task_id,
        child_task_ids: data.child_task_ids,
        certificate: data.certificate.map(|c| TaskCertificateRef {
            id: c.id,
            status: c.status,
            domains: c.domains,
        }),
        created_at: t.created_at,
        updated_at: t.updated_at,
    }
}

pub fn task_log_entry(
    m: autohttps_core::persistence::entities::task_log_entries::Model,
) -> TaskLogEntry {
    TaskLogEntry {
        id: m.id,
        task_id: m.task_id,
        seq: m.seq,
        logged_at: m.logged_at,
        level: m.level,
        message: m.message,
    }
}

// ============ acme ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AcmeAccountSummary {
    pub id: String,
    pub directory_url: String,
    pub ca_label: Option<String>,
    pub environment: Option<String>,
    pub contact_email: String,
    pub status: AcmeAccountStatus,
    pub is_default: bool,
    pub certificate_count: u64,
    pub registered_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AcmeAccountDetail {
    pub id: String,
    pub directory_url: String,
    pub ca_label: Option<String>,
    pub environment: Option<String>,
    pub contact_email: String,
    pub status: AcmeAccountStatus,
    pub is_default: bool,
    pub certificate_count: u64,
    pub registered_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub ca_account_url: Option<String>,
    pub tos_agreed: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Http01Config {
    pub domain_id: String,
    pub webroot_path: String,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSummary {
    pub id: String,
    pub task_id: String,
    pub certificate_id: String,
    pub domain_id: String,
    pub domain_hostname: Option<String>,
    pub validation_method: ValidationMethod,
    pub status: ChallengeStatus,
    pub failed_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeDetail {
    pub id: String,
    pub task_id: String,
    pub certificate_id: String,
    pub domain_id: String,
    pub domain_hostname: Option<String>,
    pub validation_method: ValidationMethod,
    pub status: ChallengeStatus,
    pub failed_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub dns_txt_name: Option<String>,
    pub dns_txt_value: Option<String>,
    pub http_file_path: Option<String>,
    pub http_file_content: Option<String>,
}

pub fn acme_account_summary(row: acme::AccountRow) -> AcmeAccountSummary {
    let a = row.account;
    AcmeAccountSummary {
        id: a.id,
        directory_url: a.directory_url,
        ca_label: a.ca_label,
        environment: a.environment,
        contact_email: a.contact_email,
        status: a.status,
        is_default: row.is_default,
        certificate_count: row.certificate_count,
        registered_at: a.registered_at,
        last_error: a.last_error,
        created_at: a.created_at,
        updated_at: a.updated_at,
    }
}

pub fn acme_account_detail(row: acme::AccountRow) -> AcmeAccountDetail {
    let is_default = row.is_default;
    let certificate_count = row.certificate_count;
    let a = row.account;
    AcmeAccountDetail {
        id: a.id,
        directory_url: a.directory_url,
        ca_label: a.ca_label,
        environment: a.environment,
        contact_email: a.contact_email,
        status: a.status,
        is_default,
        certificate_count,
        registered_at: a.registered_at,
        last_error: a.last_error,
        created_at: a.created_at,
        updated_at: a.updated_at,
        ca_account_url: a.ca_account_url,
        tos_agreed: a.tos_agreed,
    }
}

pub fn http01_config(
    m: autohttps_core::persistence::entities::http01_validation_configs::Model,
) -> Http01Config {
    Http01Config {
        domain_id: m.domain_id,
        webroot_path: m.webroot_path,
        updated_at: m.updated_at,
    }
}

/// DNS-01 提交前本地预检结果(acme api §2.3)。
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DnsPrecheckResult {
    pub propagated: bool,
    pub observed_values: Vec<String>,
}

pub fn dns_precheck_result(o: acme::DnsPrecheckOutcome) -> DnsPrecheckResult {
    DnsPrecheckResult {
        propagated: o.propagated,
        observed_values: o.observed_values,
    }
}

pub fn challenge_summary(row: acme::ChallengeRow) -> ChallengeSummary {
    let c = row.challenge;
    ChallengeSummary {
        id: c.id,
        task_id: c.task_id,
        certificate_id: row.certificate_id,
        domain_id: c.domain_id,
        domain_hostname: row.domain_hostname,
        validation_method: c.validation_method,
        status: c.status,
        failed_reason: c.failed_reason,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }
}

pub fn challenge_detail(row: acme::ChallengeRow) -> ChallengeDetail {
    let certificate_id = row.certificate_id;
    let domain_hostname = row.domain_hostname;
    let c = row.challenge;
    ChallengeDetail {
        id: c.id,
        task_id: c.task_id,
        certificate_id,
        domain_id: c.domain_id,
        domain_hostname,
        validation_method: c.validation_method,
        status: c.status,
        failed_reason: c.failed_reason,
        created_at: c.created_at,
        updated_at: c.updated_at,
        dns_txt_name: c.dns_txt_name,
        dns_txt_value: c.dns_txt_value,
        http_file_path: c.http_file_path,
        http_file_content: c.http_file_content,
    }
}

// ============ local-ca ============

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootCaSummary {
    pub id: String,
    pub name: String,
    pub status: RootCaStatus,
    pub creation_method: String,
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
    pub serial_number: Option<String>,
    pub fingerprint: Option<String>,
    pub issued_certificate_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootCaDetail {
    pub id: String,
    pub name: String,
    pub status: RootCaStatus,
    pub creation_method: String,
    pub not_before: String,
    pub not_after: String,
    pub days_until_expiry: i64,
    pub serial_number: Option<String>,
    pub fingerprint: Option<String>,
    pub issued_certificate_count: u64,
    pub created_at: String,
    pub updated_at: String,
    /// 公开材料,可内联返回(私钥永不导出,LC4)。
    pub cert_pem: String,
}

pub fn root_ca_summary(row: local_ca::RootCaRow) -> RootCaSummary {
    let r = row.root_ca;
    RootCaSummary {
        days_until_expiry: days_until(Some(&r.not_after)).unwrap_or(0),
        id: r.id,
        name: r.name,
        status: r.status,
        creation_method: r.creation_method,
        not_before: r.not_before,
        not_after: r.not_after,
        serial_number: r.serial_number,
        fingerprint: r.fingerprint,
        issued_certificate_count: row.issued_certificate_count,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub fn root_ca_detail(row: local_ca::RootCaRow) -> RootCaDetail {
    let issued_certificate_count = row.issued_certificate_count;
    let r = row.root_ca;
    RootCaDetail {
        days_until_expiry: days_until(Some(&r.not_after)).unwrap_or(0),
        id: r.id,
        name: r.name,
        status: r.status,
        creation_method: r.creation_method,
        not_before: r.not_before,
        not_after: r.not_after,
        serial_number: r.serial_number,
        fingerprint: r.fingerprint,
        issued_certificate_count,
        created_at: r.created_at,
        updated_at: r.updated_at,
        cert_pem: r.cert_pem,
    }
}
