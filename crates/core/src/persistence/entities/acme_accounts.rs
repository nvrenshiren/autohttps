//! 实体 `acme_accounts` —— ACME 账户(DB acme §2)。
use crate::domain::enums::AcmeAccountStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "acme_accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// ACME 目录端点 URL —— 唯一标定"目标 CA + 环境"。
    pub directory_url: String,
    pub ca_label: Option<String>,
    /// 生产/测试展示标签(非 §4.3 枚举,DB acme §2)。
    pub environment: Option<String>,
    pub contact_email: String,
    pub tos_agreed: bool,
    pub status: AcmeAccountStatus,
    pub ca_account_url: Option<String>,
    /// 账户密钥存储引用(敏感 AR4)—— DTO 绝不暴露。
    pub account_key_ref: Option<String>,
    pub registered_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
