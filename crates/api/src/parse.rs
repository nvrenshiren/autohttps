//! 查询参数中的枚举解析 —— 复用 serde snake_case 反序列化;非法值 → 400 validation_failed。

use crate::error::{ApiError, ApiResult};
use autohttps_core::ErrorCode;
use serde::de::DeserializeOwned;

/// 把 wire 字符串解析为枚举(如 `"self_signed"` → `IssuanceMethod::SelfSigned`)。
pub fn parse_enum<T: DeserializeOwned>(field: &str, value: &str) -> ApiResult<T> {
    serde_json::from_value::<T>(serde_json::Value::String(value.to_string())).map_err(|_| {
        ApiError::new(ErrorCode::ValidationFailed, format!("非法的 {field} 取值: {value}"))
    })
}

/// 可选枚举:None/空 → Ok(None)。
pub fn parse_enum_opt<T: DeserializeOwned>(field: &str, value: &Option<String>) -> ApiResult<Option<T>> {
    match value.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Ok(Some(parse_enum(field, s)?)),
        None => Ok(None),
    }
}

/// 逗号分隔的多值枚举(如 `status=valid,expired`);None/空 → 空 Vec。
pub fn parse_enum_list<T: DeserializeOwned>(field: &str, value: &Option<String>) -> ApiResult<Vec<T>> {
    let Some(raw) = value.as_deref() else {
        return Ok(vec![]);
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| parse_enum(field, s))
        .collect()
}
