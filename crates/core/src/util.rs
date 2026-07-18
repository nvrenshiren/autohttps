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

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn new_id_is_uuidv7() {
        let id = new_id();
        let u = Uuid::parse_str(&id).expect("应为合法 UUID 文本");
        assert_eq!(u.get_version_num(), 7, "应为 UUIDv7");
    }

    #[test]
    fn new_ids_are_time_ordered() {
        let a = new_id();
        let b = new_id();
        assert!(a < b, "UUIDv7 文本应随时间可排序({a} < {b})");
    }

    #[test]
    fn now_rfc3339_parses_back() {
        let s = now_rfc3339();
        let parsed = parse_rfc3339(&s).expect("now_rfc3339 输出应可回解析");
        let drift = (OffsetDateTime::now_utc() - parsed).whole_seconds().abs();
        assert!(drift < 5, "回解析时刻与当前漂移应极小({drift}s)");
    }

    #[test]
    fn parse_rfc3339_rejects_garbage() {
        assert!(parse_rfc3339("not-a-date").is_none());
        assert!(parse_rfc3339("").is_none());
        assert!(parse_rfc3339("2026-13-99T99:99:99Z").is_none());
    }

    #[test]
    fn days_until_none_without_not_after() {
        assert_eq!(days_until(None), None);
    }

    #[test]
    fn days_until_future_and_past() {
        // RFC3339 输出截断到秒 + 测试执行耗时,30 天整的边界可能落成 29 → 放宽一个单位
        let future = (OffsetDateTime::now_utc() + Duration::days(30))
            .format(&Rfc3339)
            .unwrap();
        let d = days_until(Some(&future)).unwrap();
        assert!((29..=30).contains(&d), "30 天后到期应剩 29~30 天,实得 {d}");

        let past = (OffsetDateTime::now_utc() - Duration::days(2))
            .format(&Rfc3339)
            .unwrap();
        assert_eq!(days_until(Some(&past)), Some(-2));
    }

    #[test]
    fn days_until_truncates_partial_days() {
        // 12 小时后到期 → 向下取整为 0 天(当天到期,非负数)
        let soon = (OffsetDateTime::now_utc() + Duration::hours(12))
            .format(&Rfc3339)
            .unwrap();
        assert_eq!(days_until(Some(&soon)), Some(0));
    }
}
