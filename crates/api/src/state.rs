//! axum 应用状态 —— 注入 core 上下文 + 全局事件广播通道。

use crate::events::ServerEvent;
use autohttps_core::CoreContext;
use tokio::sync::broadcast;

/// 广播通道容量(SSE 订阅者落后时丢弃最旧;前端重连后主动全量重取兜底,common/events §5)。
const EVENT_CHANNEL_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct AppState {
    pub ctx: CoreContext,
    pub events: broadcast::Sender<ServerEvent>,
}

impl AppState {
    pub fn new(ctx: CoreContext) -> Self {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { ctx, events }
    }
}
