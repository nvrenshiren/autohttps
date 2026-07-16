//! 实体 `tasks` —— 任务=持久队列+历史(DB tasks §2 / AR5)。`status=queued` 行即待办队列。
use crate::domain::enums::{TaskStatus, TaskTrigger, TaskType};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 关联证书 —— 软引用(证书硬删除后本行只读保留,DT3/Q2)。
    pub certificate_id: String,
    pub task_type: TaskType,
    pub trigger: TaskTrigger,
    pub status: TaskStatus,
    pub attempt_number: i32,
    /// 前序失败任务(重试链,自引用 SET NULL)。
    pub parent_task_id: Option<String>,
    pub queued_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result_summary: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
