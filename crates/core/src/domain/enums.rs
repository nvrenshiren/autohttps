//! 共享枚举 / 字典 —— **单一定义位置**(TECH §4.1 / AR2)。
//!
//! 每个枚举同时携带三重派生:
//! - `serde`(`rename_all = "snake_case"`)—— wire 值严格等于 TECH §4.3 标识;
//! - `sea_orm::DeriveActiveEnum`(`db_type = "Text"`)—— 落 SQLite TEXT 列;
//! - `ts_rs::TS` —— Rust 是唯一真相,TS 是投影(`frontend/src/bindings/`)。
//!
//! ⚠ 纪律:新增/改值必须改这里(architect 唯一入口),禁止在别处硬编码状态字面量。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 证书状态机(10 态)· certificates · TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum CertificateStatus {
    #[sea_orm(string_value = "pending_issue")]
    PendingIssue,
    #[sea_orm(string_value = "issuing")]
    Issuing,
    #[sea_orm(string_value = "issue_failed")]
    IssueFailed,
    #[sea_orm(string_value = "valid")]
    Valid,
    #[sea_orm(string_value = "expiring_soon")]
    ExpiringSoon,
    #[sea_orm(string_value = "renewing")]
    Renewing,
    #[sea_orm(string_value = "renewal_failed")]
    RenewalFailed,
    #[sea_orm(string_value = "expired")]
    Expired,
    #[sea_orm(string_value = "revoking")]
    Revoking,
    #[sea_orm(string_value = "revoked")]
    Revoked,
}

/// 签发方式 · certificates(DS3)· TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum IssuanceMethod {
    #[sea_orm(string_value = "acme")]
    Acme,
    #[sea_orm(string_value = "self_signed")]
    SelfSigned,
}

/// 任务状态机(5 态)· tasks · TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[sea_orm(string_value = "queued")]
    Queued,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "succeeded")]
    Succeeded,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// 任务类型 · tasks(§2.1)· TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    #[sea_orm(string_value = "issue")]
    Issue,
    #[sea_orm(string_value = "renew")]
    Renew,
    #[sea_orm(string_value = "revoke")]
    Revoke,
}

/// 任务触发方式 · tasks(§2.2)· TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum TaskTrigger {
    #[sea_orm(string_value = "manual")]
    Manual,
    #[sea_orm(string_value = "auto")]
    Auto,
    #[sea_orm(string_value = "cleanup")]
    Cleanup,
}

/// ACME 账户状态机(4 态)· acme · TECH §4.3
///
/// `unconfigured` 为"无账户"的概念初始态(无行即此态);持久化行不取 `unconfigured`(DB acme §2)。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum AcmeAccountStatus {
    #[sea_orm(string_value = "unconfigured")]
    Unconfigured,
    #[sea_orm(string_value = "registering")]
    Registering,
    #[sea_orm(string_value = "registered")]
    Registered,
    #[sea_orm(string_value = "registration_failed")]
    RegistrationFailed,
}

/// 验证挑战状态机(6 态)· acme · TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum ChallengeStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "awaiting_manual")]
    AwaitingManual,
    #[sea_orm(string_value = "validating")]
    Validating,
    #[sea_orm(string_value = "passed")]
    Passed,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// 验证方式类别 · acme/domains · TECH §4.3
///
/// ⚠ serde 的 `snake_case` 对 `Http01` 会产出 `http01`(数字不加下划线),与契约 `http_01` 不符;
/// 故对含数字的变体显式 `rename`,保证 wire/TS 值严格等于 §4.3 标识。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ValidationMethod {
    #[sea_orm(string_value = "http_01")]
    #[serde(rename = "http_01")]
    #[ts(rename = "http_01")]
    Http01,
    #[sea_orm(string_value = "dns_01")]
    #[serde(rename = "dns_01")]
    #[ts(rename = "dns_01")]
    Dns01,
}

/// 根 CA 状态机(2 态)· local-ca · TECH §4.3
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum, TS,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
#[serde(rename_all = "snake_case")]
pub enum RootCaStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "expired")]
    Expired,
}

/// 运行形态 · settings(DS5)· TECH §4.3
///
/// **运行时探测,非持久**(不落库);经 `GET /app-info` 暴露(common §6.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Desktop,
    Server,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// wire 值必须严格等于 TECH §4.3 标识(snake_case);前端 TS 是此处的投影,值漂移即契约破坏。
    #[test]
    fn certificate_status_wire_values() {
        let cases = [
            (CertificateStatus::PendingIssue, "pending_issue"),
            (CertificateStatus::Issuing, "issuing"),
            (CertificateStatus::IssueFailed, "issue_failed"),
            (CertificateStatus::Valid, "valid"),
            (CertificateStatus::ExpiringSoon, "expiring_soon"),
            (CertificateStatus::Renewing, "renewing"),
            (CertificateStatus::RenewalFailed, "renewal_failed"),
            (CertificateStatus::Expired, "expired"),
            (CertificateStatus::Revoking, "revoking"),
            (CertificateStatus::Revoked, "revoked"),
        ];
        assert_eq!(cases.len(), 10, "证书状态机为 10 态");
        for (variant, wire) in cases {
            assert_eq!(
                serde_json::to_value(variant).unwrap(),
                serde_json::json!(wire)
            );
            assert_eq!(variant.to_value(), wire, "DB 值应等于 wire 值");
        }
    }

    #[test]
    fn task_and_challenge_wire_values() {
        assert_eq!(serde_json::to_value(TaskStatus::Queued).unwrap(), "queued");
        assert_eq!(
            serde_json::to_value(TaskStatus::Cancelled).unwrap(),
            "cancelled"
        );
        assert_eq!(serde_json::to_value(TaskType::Renew).unwrap(), "renew");
        assert_eq!(
            serde_json::to_value(TaskTrigger::Cleanup).unwrap(),
            "cleanup"
        );
        assert_eq!(
            serde_json::to_value(ChallengeStatus::AwaitingManual).unwrap(),
            "awaiting_manual"
        );
        assert_eq!(
            serde_json::to_value(AcmeAccountStatus::RegistrationFailed).unwrap(),
            "registration_failed"
        );
        assert_eq!(
            serde_json::to_value(RootCaStatus::Active).unwrap(),
            "active"
        );
        assert_eq!(serde_json::to_value(RunMode::Desktop).unwrap(), "desktop");
    }

    #[test]
    fn validation_method_uses_explicit_rename() {
        // serde snake_case 对 Http01 会产出 http01(数字不加下划线);契约要求 http_01/dns_01。
        assert_eq!(
            serde_json::to_value(ValidationMethod::Http01).unwrap(),
            "http_01"
        );
        assert_eq!(
            serde_json::to_value(ValidationMethod::Dns01).unwrap(),
            "dns_01"
        );
        assert_eq!(ValidationMethod::Http01.to_value(), "http_01");
        assert_eq!(ValidationMethod::Dns01.to_value(), "dns_01");
    }

    #[test]
    fn wire_values_roundtrip() {
        let v: CertificateStatus =
            serde_json::from_str("\"expiring_soon\"").expect("wire 值应可反序列化");
        assert_eq!(v, CertificateStatus::ExpiringSoon);
        assert!(serde_json::from_str::<CertificateStatus>("\"ExpiringSoon\"").is_err());
    }
}
