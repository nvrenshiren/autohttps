//! axum handlers —— 按模块拆分。里程碑1:读取/CRUD 真实;crypto/executor 动作打桩 501。

pub mod acme;
pub mod app_info;
pub mod certificates;
pub mod dashboard;
pub mod domains;
pub mod events;
pub mod local_ca;
pub mod settings;
pub mod tasks;
