//! 持久化层 —— SQLite 访问 + 实体 + 迁移(ARCHITECTURE §5,落 `crates/core/src/persistence/`)。
pub mod db;
pub mod entities;
pub mod migration;
