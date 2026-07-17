//! # autohttps-core —— 领域核心库(业务唯一真相)
//!
//! 三层 crate 的底座(ARCHITECTURE §2 / AR1):5 台状态机枚举(单一真相 §4.3)、SeaORM 实体 +
//! 迁移、SQLite 访问、敏感数据存储、用例服务。两形态(server/desktop)共用;api 依赖之。
//!
//! 进度:CRUD/查询、自签 CA、执行器、age 加密、扫描器(到期+自动续签)、领域事件(→ SSE)、
//! ACME 全线(账户注册 / 挑战推进 / 取证 / 续签 / 吊销;DNS-01 手动挂起→confirm)均已实现。

pub mod ca;
pub mod domain;
pub mod persistence;
pub mod scan;
pub mod secrets;
pub mod services;
pub mod util;

pub use domain::enums;
pub use domain::error::{CoreError, CoreResult, ErrorCode};
pub use domain::events::DomainEvent;
pub use services::context::CoreContext;

/// 便捷 re-export:两形态 bin 的 boot 入口。
pub use persistence::db;
pub use services::boot;
