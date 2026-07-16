//! 内嵌 SPA(rust-embed)—— 前端 Vite 构建产物内嵌进可执行文件,axum 以静态资源提供(ARCHITECTURE §4.3)。
//!
//! 内嵌目录为 `frontend/dist`(需先 `npm run build` 才有真实产物)。fresh clone 未构建前端时,
//! `build.rs` 于编译期兜底创建空占位目录,使 `cargo build` 不依赖先跑前端;此时目录为空,运行期
//! 静态资源全部未命中 → 回退提示"请先构建前端"。命中时未匹配路径回退 `index.html`(SPA fallback)。

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist"]
struct Assets;

fn serve(path: &str) -> Option<Response> {
    let asset = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        (
            [(header::CONTENT_TYPE, mime.as_ref().to_string())],
            asset.data.into_owned(),
        )
            .into_response(),
    )
}

/// 静态资源 + SPA fallback。API 路由不命中此处(先于 fallback 注册)。
pub async fn spa_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(resp) = serve(path) {
        return resp;
    }
    // 未命中 → 回退 index.html(前端路由接管)
    if let Some(resp) = serve("index.html") {
        return resp;
    }
    (
        StatusCode::NOT_FOUND,
        "SPA 未构建:请先 `cd frontend && npm install && npm run build`",
    )
        .into_response()
}
