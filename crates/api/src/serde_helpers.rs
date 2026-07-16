//! serde 辅助 —— PATCH 语义的 double-option:区分"字段缺省(不改)"与"字段为 null(清空)"。

use serde::{Deserialize, Deserializer};

/// 字段缺省 → `None`(不改);字段为 `null` → `Some(None)`(清空);字段有值 → `Some(Some(v))`(设值)。
///
/// 配合 `#[serde(default, deserialize_with = "double_option")]`:缺省时 serde 用 `default`(None),
/// 不调用本函数;字段出现(null 或值)时本函数被调用。
pub fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}
