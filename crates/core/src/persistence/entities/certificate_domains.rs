//! 实体 `certificate_domains` —— 证书 ↔ 域名 SAN 关联(复合 PK)(DB certificates §3)。
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "certificate_domains")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub certificate_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub domain_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
