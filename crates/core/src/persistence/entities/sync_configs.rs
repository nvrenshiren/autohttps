//! 实体 `sync_configs` —— WebDAV 备份同步配置单例(sync §配置)。`id='webdav'` 哨兵,仅一行。
//!
//! 口令不入库:`password_ref` 为 SecretStore 引用键(密文落 `secrets/`)。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 单例哨兵主键值。
pub const SINGLETON_ID: &str = "webdav";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_configs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// 远端目录完整 URL(含远程路径,已归一末尾斜杠)。
    pub base_url: String,
    pub username: String,
    /// WebDAV 口令的 SecretStore 引用键(口令本身绝不入库)。
    pub password_ref: Option<String>,
    /// 最近一次成功备份时刻(RFC3339;NULL=从未成功)。
    pub last_backup_at: Option<String>,
    /// 最近一次备份动作结果:`success` / `failed`(展示用;NULL=从未备份)。
    pub last_backup_result: Option<String>,
    /// 最近失败原因(成功时清空)。
    pub last_backup_error: Option<String>,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
