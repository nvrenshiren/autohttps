//! 实体 `task_log_entries` —— 逐条执行日志(DB tasks §3)。**脱敏,绝不含密钥**(AR4/L6)。
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "task_log_entries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_id: String,
    /// 任务内序号(有序回放)。
    pub seq: i32,
    pub logged_at: String,
    /// info / warn / error(与 tracing 对齐;局部属性,非 §4.3 枚举)。
    pub level: String,
    /// 脱敏日志内容 —— 私钥/账户密钥/根 CA 私钥绝不写入。
    pub message: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
