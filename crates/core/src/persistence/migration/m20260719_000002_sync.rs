//! 迁移 000002 —— 新增 `sync_configs` 单例表(WebDAV 备份同步配置)。
//!
//! 口令不入库:仅存 `password_ref`(SecretStore 引用键,AR4 同口径)。
//! 远程路径已并入 base_url(由调用方归一),不单列。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const SCHEMA: &str = r#"
CREATE TABLE sync_configs (
  id TEXT PRIMARY KEY NOT NULL DEFAULT 'webdav' CHECK (id = 'webdav'),
  base_url TEXT NOT NULL,
  username TEXT NOT NULL,
  password_ref TEXT,
  last_backup_at TEXT,
  last_backup_result TEXT,
  last_backup_error TEXT,
  updated_at TEXT NOT NULL
);
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for stmt in SCHEMA.split(';') {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            db.execute_unprepared(s).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS sync_configs")
            .await?;
        Ok(())
    }
}
