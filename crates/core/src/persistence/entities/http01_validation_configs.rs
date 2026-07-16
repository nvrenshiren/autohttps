//! 实体 `http01_validation_configs` —— HTTP-01 webroot 执行配置(DB acme §3),按域名 1:0..1。
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "http01_validation_configs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub domain_id: String,
    pub webroot_path: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
