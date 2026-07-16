//! 迁移器(sea-orm-migration)—— boot 时由 bin 调用 `Migrator::up(&db, None)`。
use sea_orm_migration::prelude::*;

mod m20260716_000001_init;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260716_000001_init::Migration)]
    }
}
