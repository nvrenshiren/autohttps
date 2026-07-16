//! 服务上下文 —— 两形态 bin 在 boot 时构造,注入 api 层。
use crate::domain::enums::RunMode;
use crate::secrets::SecretStore;
use sea_orm::DatabaseConnection;
use std::path::PathBuf;

/// core 用例服务的运行上下文(DB + 密钥存储 + 运行形态)。
#[derive(Clone)]
pub struct CoreContext {
    pub db: DatabaseConnection,
    pub secrets: SecretStore,
    pub data_dir: PathBuf,
    /// 运行形态 —— 运行载体探测(server / desktop),经 `GET /app-info` 暴露(SF4/DS5)。
    pub run_mode: RunMode,
    pub app_version: String,
}

impl CoreContext {
    pub fn new(
        db: DatabaseConnection,
        data_dir: PathBuf,
        run_mode: RunMode,
        app_version: String,
    ) -> Self {
        let secrets = SecretStore::new(&data_dir);
        Self { db, secrets, data_dir, run_mode, app_version }
    }
}
