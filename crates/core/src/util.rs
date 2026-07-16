//! 时间(RFC3339 UTC,TECH §3.5)与 ID(UUIDv7,决策10)工具。
//!
//! 时间列在库内以 `TEXT·RFC3339` 存储(DB _overview §1),与 wire 表示一致 —— 直接存字符串,
//! 避免时区/整型漂移(protocolLint L3)。计算类字段(`daysUntilExpiry`)在服务层解析后相对
//! 服务器当前 UTC 时刻算出(common §1)。

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

/// 生成新的 UUIDv7 文本 ID(时间可排序、对外不透明、不复用)。
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// 当前 UTC 时刻的 RFC3339 字符串(如 `2026-07-16T08:00:00Z`)。
pub fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

/// 解析 RFC3339 字符串为 `OffsetDateTime`;非法返回 `None`(容错,不 panic)。
pub fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// 计算 `notAfter` 相对服务器当前时刻的剩余天数(向下取整;已过期为负)。
///
/// 无有效期(未签发)→ `None`(common §1)。
pub fn days_until(not_after: Option<&str>) -> Option<i64> {
    let na = parse_rfc3339(not_after?)?;
    let now = OffsetDateTime::now_utc();
    Some((na - now).whole_days())
}
