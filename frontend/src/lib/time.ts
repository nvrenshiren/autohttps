/**
 * 时间显示统一(设计 §10-H11):数据契约为 RFC3339 UTC(TECH §3.5);**显示**为「相对时间 +
 * Tooltip 绝对时间」;绝对时间用 mono。
 */

/** 绝对时间(UTC,mono 展示)。null → "—"。 */
export function absoluteUtc(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  // 形如 2026-07-16 08:00:00 UTC
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())} ${p(
    d.getUTCHours(),
  )}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())} UTC`;
}

/** 相对时间(中文)。null → "—"。 */
export function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const diffMs = d.getTime() - Date.now();
  const past = diffMs <= 0;
  const abs = Math.abs(diffMs);
  const min = 60_000;
  const hour = 60 * min;
  const day = 24 * hour;

  let label: string;
  if (abs < min) label = "刚刚";
  else if (abs < hour) label = `${Math.floor(abs / min)} 分钟`;
  else if (abs < day) label = `${Math.floor(abs / hour)} 小时`;
  else label = `${Math.floor(abs / day)} 天`;

  if (label === "刚刚") return label;
  return past ? `${label}前` : `${label}后`;
}

/** 剩余天数标签(daysUntilExpiry;已过期为负)。 */
export function daysLabel(days: number | null | undefined): string {
  if (days === null || days === undefined) return "未签发";
  if (days < 0) return `已过期 ${Math.abs(days)} 天`;
  if (days === 0) return "今天到期";
  return `${days} 天后到期`;
}
