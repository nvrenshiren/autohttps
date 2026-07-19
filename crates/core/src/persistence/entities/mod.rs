//! SeaORM 实体(schema 单一真相)—— 11 表,落 `crates/core/src/persistence/`(ARCHITECTURE §5)。
//!
//! 时间列以 `TEXT·RFC3339` 存储(String);枚举列走 `DeriveActiveEnum`(Text);
//! 敏感数据只存 `*_ref`(AR4)。关系用显式查询表达(里程碑1 不建 SeaORM Relation)。

pub mod acme_accounts;
pub mod certificate_domains;
pub mod certificates;
pub mod challenges;
pub mod domains;
pub mod http01_validation_configs;
pub mod internal_cert_revocations;
pub mod root_cas;
pub mod settings;
pub mod sync_configs;
pub mod task_log_entries;
pub mod tasks;
