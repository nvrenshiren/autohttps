//! `GET /events` —— 全局 SSE 流(common/events.md)。
//!
//! 订阅 core 的领域事件广播(`CoreContext.events`),经 [`crate::events::to_server_event`] 映射为
//! wire `ServerEvent` 后逐帧推送(`event:` 为类型、`data:` 为 JSON 包络);并以 KeepAlive 周期发送
//! 注释行维持长连接。订阅者落后被丢弃(Lagged)时忽略该错误,前端 `onopen` 重连后主动全量重取兜底。

use crate::events::to_server_event;
use crate::state::AppState;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

pub async fn stream(
    State(st): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = st.ctx.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(ev) => {
            let se = to_server_event(&ev);
            let data = serde_json::to_string(&se).unwrap_or_default();
            Some(Ok(Event::default().event(se.event_type.as_str()).data(data)))
        }
        // 订阅者落后被丢弃(Lagged):忽略;前端 onopen 重连后主动全量重取兜底(common/events §5)。
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
