//! `GET /events` —— 全局 SSE 流(common/events.md)。
//!
//! 里程碑1:**心跳骨架**。订阅广播通道(当前 core 尚未发事件,执行器/扫描器打桩),并以 KeepAlive
//! 周期发送注释行维持长连接。实现期由 core 服务经 `AppState.events` 发出状态变更事件。

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
    let rx = st.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(ev) => {
            let data = serde_json::to_string(&ev).unwrap_or_default();
            Some(Ok(Event::default().event(ev.event_type.as_str()).data(data)))
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
