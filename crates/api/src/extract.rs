//! 请求体提取器 —— 把 JSON 解析失败(结构/类型错)统一映射为 `400 validation_failed`(common §4.2),
//! 使错误包络在全端一致(而非 axum 默认的裸文本 400)。

use crate::error::ApiError;
use autohttps_core::ErrorCode;
use axum::extract::{FromRequest, Request};
use axum::Json;
use serde::de::DeserializeOwned;

pub struct JsonBody<T>(pub T);

impl<T, S> FromRequest<S> for JsonBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(JsonBody(value)),
            Err(rejection) => Err(ApiError::new(
                ErrorCode::ValidationFailed,
                rejection.body_text(),
            )),
        }
    }
}
