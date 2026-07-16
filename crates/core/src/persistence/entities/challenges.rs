//! 实体 `challenges` —— 验证挑战记录(DB acme §4)。挑战状态机 6 态。
use crate::domain::enums::{ChallengeStatus, ValidationMethod};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "challenges")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 所属签发/续签任务(FK CASCADE)。
    pub task_id: String,
    /// 验证对象域名 —— 软引用(历史挑战不随域名删除受阻)。
    pub domain_id: String,
    pub validation_method: ValidationMethod,
    pub status: ChallengeStatus,
    /// DNS-01 待添加 TXT 记录名(展示供复制;非敏感)。
    pub dns_txt_name: Option<String>,
    pub dns_txt_value: Option<String>,
    pub http_file_path: Option<String>,
    pub http_file_content: Option<String>,
    pub authorization_url: Option<String>,
    pub failed_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
