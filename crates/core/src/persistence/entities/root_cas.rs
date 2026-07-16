//! 实体 `root_cas` —— 自签根 CA(DB local-ca §2)。根 CA 状态机 2 态。
use crate::domain::enums::RootCaStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "root_cas")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub status: RootCaStatus,
    /// `created` / `imported`(局部 2 值属性,非 §4.3 枚举)。
    pub creation_method: String,
    pub not_before: String,
    pub not_after: String,
    pub serial_number: Option<String>,
    pub fingerprint: Option<String>,
    /// 根 CA 证书本体 PEM(公开材料,内联存储)。
    pub cert_pem: String,
    /// 根 CA 私钥存储引用(敏感级最高,AR4)—— DTO 绝不暴露,永不导出(LC4)。
    pub private_key_ref: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
