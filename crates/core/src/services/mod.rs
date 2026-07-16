//! 用例服务层(编排状态机 + 持久化)。api 层调用这些服务、映射为 DTO。
//!
//! 里程碑1:domains / certificates / settings / dashboard 及各模块 list+detail 为真实实现;
//! 依赖 ACME/CA/执行器/扫描器的动作打桩(见各 handler 与 `boot`)。

pub mod acme;
pub mod boot;
pub mod certificates;
pub mod context;
pub mod dashboard;
pub mod domains;
pub mod executor;
pub mod local_ca;
pub mod pagination;
pub mod settings;
pub mod tasks;

pub use context::CoreContext;
