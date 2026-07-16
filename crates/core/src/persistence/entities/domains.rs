//! 实体 `domains` —— 域名核心(DB domains §2)。无独立状态机。
use crate::domain::enums::ValidationMethod;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "domains")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 身份;同实例唯一;不可编辑(改名=删+增)。
    #[sea_orm(unique)]
    pub hostname: String,
    pub is_wildcard: bool,
    pub validation_method: Option<ValidationMethod>,
    pub group_name: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
