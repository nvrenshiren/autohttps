---
name: enum-digit-wire-values
description: §4.3 枚举中含数字的变体(如 http_01/dns_01)必须三处显式 rename,否则 wire 值与契约不符
metadata:
  type: project
---

`crates/core/src/domain/enums.rs` 里,**含数字的枚举变体不能只靠 `rename_all`**。

**Why:** serde 的 `rename_all = "snake_case"` 对 `Http01` 产出 `http01`(数字前不加下划线),但 TECH §4.3
契约要求 wire 值严格等于 `http_01` / `dns_01`。首次实现 `ValidationMethod` 就踩了这个坑(curl 报
`unknown variant http_01, expected http01`)。

**How to apply:** 对任何含数字的 §4.3 枚举变体,三处都要显式 rename(已在 `ValidationMethod` 做全):
- serde:`#[serde(rename = "http_01")]`(wire/JSON)
- SeaORM:`#[sea_orm(string_value = "http_01")]`(DB 落值)
- ts-rs:`#[ts(rename = "http_01")]`(TS 投影)

纯字母变体(`pending_issue`、`self_signed`、`awaiting_manual` 等)`rename_all="snake_case"` 正常,无需
逐个 rename。验证:`serde_json` 反序列化该 wire 值应成功;前端 `frontend/src/bindings/index.ts` 手写投影
需与之一致。
