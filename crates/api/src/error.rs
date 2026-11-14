//! 错误包络(common §4)—— `{ "error": { "code", "message", "details } }`,`code` 为 snake_case。
//!
//! `ApiError` 包裹 core 的 `CoreError`,`IntoResponse` 输出统一包络;HTTP 状态由 `ErrorCode` 决定。

use autohttps_core::{CoreError, ErrorCode};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

pub struct ApiError(pub CoreError);

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        ApiError(e)
    }
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ApiError(CoreError::new(code, message))
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorPayload,
}

#[derive(Serialize)]
struct ErrorPayload {
    code: ErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = ErrorBody {
            error: ErrorPayload {
                code: self.0.code,
                message: self.0.message,
                details: self.0.details,
            },
        };
        (status, Json(body)).into_response()
    }
}

/// api 层内部结果别名。
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn validation_error_keeps_status_and_wire_envelope() {
        let response = ApiError::new(ErrorCode::ValidationFailed, "输入无效").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse JSON");
        assert_eq!(json["error"]["code"], "validation_failed");
        assert_eq!(json["error"]["message"], "输入无效");
        assert!(json["error"].get("details").is_none());
    }
}
