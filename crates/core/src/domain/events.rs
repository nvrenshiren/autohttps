//! 领域事件(实时推送的**语义源**)—— core 服务在状态变更后 `emit`,api 层订阅并映射为 SSE
//! `ServerEvent`(wire `EventType` + camelCase payload 单一定义在 `crates/api`,common/events.md §4)。
//!
//! 分层:core 只发"语义事件"(不感知 SSE/wire 格式);api 订阅并翻译为对外契约。payload 极简
//! (仅标识 + 判别字段,不搬整实体、不含密钥 L6)——由 api 映射时构造。

use crate::domain::enums::{CertificateStatus, RootCaStatus, TaskStatus};

/// core 侧语义事件。变体覆盖本切片实际发出的实时信号(证书/任务/根 CA 状态推进、任务日志、
/// dashboard 聚合)。acme(挑战/账户)事件待 ACME 接入后补(执行仍桩,故此处暂不定义)。
#[derive(Debug, Clone)]
pub enum DomainEvent {
    /// 证书状态机任一流转(执行器结果 / 扫描 T6/T10 / 取消回退)。
    CertificateStatusChanged { certificate_id: String, status: CertificateStatus },
    /// 任务状态机流转(入队 / 开始 / 终态 / 派生)。
    TaskStatusChanged { task_id: String, certificate_id: String, status: TaskStatus },
    /// 任务执行中新增一条日志(进度)。
    TaskLogAppended { task_id: String, seq: i32 },
    /// 根 CA 状态机流转(扫描 L3:active→expired)。
    RootCaStatusChanged { root_ca_id: String, status: RootCaStatus },
    /// 红点更新:待处理集合 / 三指标变化时的**粗粒度合并信号**;`pending_count` 口径同 `GET /dashboard`。
    DashboardChanged { pending_count: i64 },
}
