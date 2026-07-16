//! axum 应用状态 —— 注入 core 上下文。
//!
//! 实时事件源为 core 的领域事件广播(`ctx.events`);SSE 处理器直接订阅之并映射为 wire 事件
//! (见 handlers/events.rs)。api 不再自建第二条广播通道 —— 单一事件源,分层不倒挂。

use autohttps_core::CoreContext;

#[derive(Clone)]
pub struct AppState {
    pub ctx: CoreContext,
}

impl AppState {
    pub fn new(ctx: CoreContext) -> Self {
        Self { ctx }
    }
}
