//! 全局 SSE 事件类型(common/events.md §4)—— **wire 契约单一定义**,导出 TS。
//!
//! `GET /events`(见 handlers/events.rs)订阅 core 的领域事件广播(`CoreContext.events`),经
//! [`to_server_event`] 映射为本模块的 `ServerEvent`(camelCase payload)后推给前端。core 只发语义
//! 事件(`DomainEvent`,不感知 wire),api 独占 wire 契约 —— 分层不倒挂。

use autohttps_core::domain::events::DomainEvent;
use autohttps_core::util::now_rfc3339;
use serde::Serialize;
use serde_json::json;
use ts_rs::TS;

/// 事件类型(与 SSE `event:` 字段一致)。状态字段取值严格取 §4.3 wire 值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    CertificateStatusChanged,
    TaskStatusChanged,
    TaskLogAppended,
    ChallengeStatusChanged,
    AcmeAccountStatusChanged,
    RootCaStatusChanged,
    DashboardChanged,
}

impl EventType {
    /// SSE `event:` 字段名(snake_case)。
    pub fn as_str(self) -> &'static str {
        use EventType::*;
        match self {
            CertificateStatusChanged => "certificate_status_changed",
            TaskStatusChanged => "task_status_changed",
            TaskLogAppended => "task_log_appended",
            ChallengeStatusChanged => "challenge_status_changed",
            AcmeAccountStatusChanged => "acme_account_status_changed",
            RootCaStatusChanged => "root_ca_status_changed",
            DashboardChanged => "dashboard_changed",
        }
    }
}

/// 统一事件包络(payload 极简:仅标识 + 判别字段,不搬运整实体,common/events §3)。
#[derive(Debug, Clone, Serialize)]
pub struct ServerEvent {
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// 事件发生时间(RFC3339 UTC)。
    pub at: String,
    pub payload: serde_json::Value,
}

/// 把 core 的语义事件映射为对外 SSE 包络。payload 键 **camelCase**(同 REST DTO),状态字段取
/// 枚举 wire 值(serde snake_case,§4.3);**绝不含 `*_ref`/密钥**(L6)。
pub fn to_server_event(ev: &DomainEvent) -> ServerEvent {
    let (event_type, payload) = match ev {
        DomainEvent::CertificateStatusChanged { certificate_id, status } => (
            EventType::CertificateStatusChanged,
            json!({ "certificateId": certificate_id, "status": status }),
        ),
        DomainEvent::TaskStatusChanged { task_id, certificate_id, status } => (
            EventType::TaskStatusChanged,
            json!({ "taskId": task_id, "certificateId": certificate_id, "status": status }),
        ),
        DomainEvent::TaskLogAppended { task_id, seq } => {
            (EventType::TaskLogAppended, json!({ "taskId": task_id, "seq": seq }))
        }
        DomainEvent::RootCaStatusChanged { root_ca_id, status } => (
            EventType::RootCaStatusChanged,
            json!({ "rootCaId": root_ca_id, "status": status }),
        ),
        DomainEvent::AcmeAccountStatusChanged { account_id, status } => (
            EventType::AcmeAccountStatusChanged,
            json!({ "accountId": account_id, "status": status }),
        ),
        DomainEvent::ChallengeStatusChanged { challenge_id, task_id, domain_id, status } => (
            EventType::ChallengeStatusChanged,
            json!({
                "challengeId": challenge_id,
                "taskId": task_id,
                "domainId": domain_id,
                "status": status,
            }),
        ),
        DomainEvent::DashboardChanged { pending_count } => {
            (EventType::DashboardChanged, json!({ "pendingCount": pending_count }))
        }
    };
    ServerEvent { event_type, at: now_rfc3339(), payload }
}
