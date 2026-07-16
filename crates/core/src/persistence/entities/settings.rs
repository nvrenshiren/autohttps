//! 实体 `settings` —— 全局配置单例(DB settings §2)。`id='global'` 哨兵,仅一行。
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 单例哨兵主键值。
pub const SINGLETON_ID: &str = "global";

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub renewal_advance_days: i32,
    pub auto_renew_enabled: bool,
    /// 默认 ACME 账户指向(FK SET NULL,可空)。
    pub default_acme_account_id: Option<String>,
    /// 仅桌面:开机自启开关。
    pub autostart_enabled: Option<bool>,
    /// 仅服务器:Web UI 监听地址。
    pub listen_address: Option<String>,
    /// 仅服务器:Web UI 监听端口。
    pub listen_port: Option<i32>,
    /// 数据存储根路径 —— 运行期只读、不可改、无迁移(SF5/DEC3)。
    pub data_storage_path: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
