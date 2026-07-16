//! 服务上下文 —— 两形态 bin 在 boot 时构造,注入 api 层。
use crate::domain::enums::RunMode;
use crate::domain::events::DomainEvent;
use crate::secrets::SecretStore;
use sea_orm::DatabaseConnection;
use std::path::PathBuf;
use tokio::sync::broadcast;

/// 领域事件广播通道容量。SSE 订阅者落后即丢弃最旧(Lagged);前端重连后主动全量重取兜底
/// (common/events §5)。
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// core 用例服务的运行上下文(DB + 密钥存储 + 运行形态 + 事件广播)。
#[derive(Clone)]
pub struct CoreContext {
    pub db: DatabaseConnection,
    pub secrets: SecretStore,
    pub data_dir: PathBuf,
    /// 运行形态 —— 运行载体探测(server / desktop),经 `GET /app-info` 暴露(SF4/DS5)。
    pub run_mode: RunMode,
    pub app_version: String,
    /// 领域事件广播源 —— core 服务经 [`CoreContext::emit`] 发出状态变更;api 订阅 → 映射为 SSE。
    pub events: broadcast::Sender<DomainEvent>,
}

impl CoreContext {
    pub fn new(
        db: DatabaseConnection,
        data_dir: PathBuf,
        run_mode: RunMode,
        app_version: String,
    ) -> Self {
        let secrets = SecretStore::new(&data_dir);
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { db, secrets, data_dir, run_mode, app_version, events }
    }

    /// 发出一个领域事件(**尽力而为**:无 SSE 订阅者时静默丢弃,不阻塞业务)。
    pub fn emit(&self, event: DomainEvent) {
        let _ = self.events.send(event);
    }
}
