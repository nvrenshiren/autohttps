//! 实体 `internal_cert_revocations` —— 根 CA 名下内网证书作废记录(DB local-ca §3)。
//! rcgen 无 CRL/OCSP,MVP 作废=本地作废记录;独立于证书生命周期长存。
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "internal_cert_revocations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 签发该内网证书的根 CA(FK RESTRICT)。
    pub root_ca_id: String,
    /// 被作废内网证书序列号(作废清单键)。`(root_ca_id, serial_number)` 唯一。
    pub serial_number: String,
    /// 对应内网证书 —— 软引用(可能已删除)。
    pub certificate_id: Option<String>,
    pub revoked_at: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
