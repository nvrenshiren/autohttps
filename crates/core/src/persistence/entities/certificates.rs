//! 实体 `certificates` —— 证书核心(DB certificates §2)。全局枢纽。
use crate::domain::enums::{CertificateStatus, IssuanceMethod};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "certificates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub issuance_method: IssuanceMethod,
    pub status: CertificateStatus,
    pub acme_account_id: Option<String>,
    pub root_ca_id: Option<String>,
    pub serial_number: Option<String>,
    pub fingerprint: Option<String>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub issued_at: Option<String>,
    /// 证书文件(公开)存储引用 —— DTO 绝不暴露(AR4)。
    pub cert_pem_ref: Option<String>,
    /// 私钥存储引用(敏感 AR4)—— DTO/日志绝不暴露。
    pub private_key_ref: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
