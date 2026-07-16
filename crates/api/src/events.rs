//! 全局 SSE 事件类型(common/events.md §4)—— 单一定义,导出 TS。
//!
//! 里程碑1:`GET /events` 为**心跳骨架**(见 handlers/events.rs),core 尚未发事件(执行器/扫描器打桩)。
//! `EventType` 与广播通道已就位,实现期由 core 服务经 `AppState.events` 发出。

use serde::Serialize;
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
