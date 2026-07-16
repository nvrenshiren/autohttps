//! SQLite 连接(每实例一库,WAL)+ 迁移驱动(DB _overview §1 / boot 序列 ARCHITECTURE §7)。
//!
//! 经 sqlx `SqliteConnectOptions` 设 `journal_mode=WAL` + `foreign_keys=ON`(对池内每条连接生效),
//! 再交给 SeaORM。FK 是"兜底"约束(业务不变量另由服务层强制,DB 各模块 §2.3)。

use crate::persistence::migration::Migrator;
use sea_orm::{DatabaseConnection, SqlxSqliteConnector};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;

/// 打开(必要时创建)SQLite 库并返回 SeaORM 连接。
pub async fn connect(db_path: &Path) -> anyhow::Result<DatabaseConnection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;

    Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

/// 执行全部未应用迁移(幂等)。
pub async fn migrate(db: &DatabaseConnection) -> anyhow::Result<()> {
    Migrator::up(db, None).await?;
    Ok(())
}
