//! 用例服务层(编排状态机 + 持久化)。api 层调用这些服务、映射为 DTO。
//!
//! domains / certificates / settings / dashboard / acme / local-ca 及各模块 list+detail 均为真实实现;
//! 依赖 CA/ACME/执行器/扫描器的动作(签发/续签/吊销/注册/挑战推进)已全部落地(见各服务与 `executor`)。

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
