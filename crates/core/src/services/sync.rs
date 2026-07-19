//! WebDAV 备份同步服务 —— 配置持久化(口令经 SecretStore)+ 备份/恢复/列远端编排。
//!
//! - 配置单例(`sync_configs` id='webdav');口令只经 `password` 入参写入,读取永远不回传;
//! - 备份 = `sync::backup::pack_backup` 打包加密 → `webdav::upload`,结果落 last_backup_*;
//! - 恢复 = `webdav::download` → `parse_backup`(内存校验)→ **在线写回**(ATTACH 逐表替换,
//!   不挪动活跃库文件,规避 Windows 文件锁)→ 返回 `requires_restart`。
//!
//! 安全口径(AR4/L6):口令/备份口令绝不入库、绝不入日志;错误 message 不带 URL 凭据段。

use crate::domain::error::{CoreError, CoreResult, ErrorCode};
use crate::persistence::entities::sync_configs;
use crate::services::context::CoreContext;
use crate::sync::{backup, webdav};
use crate::util::{new_id, now_rfc3339};
use sea_orm::*;

/// 备份文件名前缀(远端列出时按名过滤,时间戳命名 → 字典序即时间序)。
const BACKUP_NAME_PREFIX: &str = "autohttps-backup-";
/// 现场归档目录名(恢复前把当前 db/secrets 挪到这里,可回滚)。
const ARCHIVE_DIR_NAME: &str = "restore-archive";

/// 对外暴露的配置视图(**不含口令**;`password_set` 告知前端是否已存口令)。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfigView {
    pub configured: bool,
    /// 服务器地址(不含远程目录;展示/回填用)。
    pub server_url: Option<String>,
    /// 远程目录(相对服务器根,备份文件与其他项目隔离;展示/回填用)。
    pub remote_dir: Option<String>,
    /// 拼好的完整远端目录 URL(server_url + remote_dir;实际请求目标)。
    pub base_url: Option<String>,
    pub username: Option<String>,
    pub password_set: bool,
    pub last_backup_at: Option<String>,
    pub last_backup_result: Option<String>,
    pub last_backup_error: Option<String>,
}

/// 保存配置输入;`password: None` = 保留已存口令,`Some("")` = 清除口令。
#[derive(Debug, Default)]
pub struct SaveSyncConfigInput {
    /// 服务器地址(如 `https://dav.example.com/dav`;不带备份目录)。
    pub server_url: String,
    /// 远程目录(缺省 `autohttps/`;备份与同一 WebDAV 上其他项目隔离)。
    pub remote_dir: Option<String>,
    pub username: String,
    pub password: Option<String>,
}

/// 远端备份文件项(展示用)。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBackupItem {
    pub name: String,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

/// 恢复结果(写回成功后进程需重启以重连 DB/密钥缓存)。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub restored_from: String,
    pub backup_created_at: String,
    pub secrets_restored: u32,
    pub requires_restart: bool,
}

/// 默认远程目录(避免备份散在 WebDAV 根目录、与其他项目混杂)。
pub const DEFAULT_REMOTE_DIR: &str = "autohttps";

/// 校验并归一服务器地址:必须 http(s),不带末尾斜杠(目录由 remote_dir 拼接)。
fn normalize_server_url(raw: &str) -> CoreResult<String> {
    let url = raw.trim().trim_end_matches('/');
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(CoreError::new(
            ErrorCode::ValidationFailed,
            "WebDAV 服务器地址须以 http:// 或 https:// 开头",
        ));
    }
    Ok(url.to_string())
}

/// 归一远程目录:剥首尾斜杠;空 → 默认目录;拒绝上跳/反斜杠(防路径穿越出服务器根)。
fn normalize_remote_dir(raw: Option<&str>) -> CoreResult<String> {
    let dir = raw.unwrap_or(DEFAULT_REMOTE_DIR).trim().trim_matches('/');
    if dir.is_empty() {
        return Ok(DEFAULT_REMOTE_DIR.to_string());
    }
    if dir.contains("..") || dir.contains('\\') {
        return Err(CoreError::new(
            ErrorCode::ValidationFailed,
            "远程目录不能包含 `..` 或反斜杠",
        ));
    }
    Ok(dir.to_string())
}

/// 由服务器地址 + 远程目录拼完整 base_url(末尾带斜杠,webdav 层拼文件名用)。
fn join_base_url(server_url: &str, remote_dir: &str) -> String {
    format!("{server_url}/{remote_dir}/")
}

/// 从完整 base_url 拆回(服务器地址, 远程目录)供展示/回填。
/// 拆不出目录段(历史数据/手填根目录)时,目录段为空串(前端显示为根目录)。
fn split_base_url(base_url: &str) -> (String, String) {
    let trimmed = base_url.trim_end_matches('/');
    match trimmed.rfind('/') {
        // 排除 scheme 的 `//`:`https://host` 整体算服务器地址,目录为空
        Some(i) if i > trimmed.find("://").map(|s| s + 2).unwrap_or(0) => {
            (trimmed[..i].to_string(), trimmed[i + 1..].to_string())
        }
        _ => (trimmed.to_string(), String::new()),
    }
}

/// 读配置单例;未配置返回 None(读路径不隐式建行,避免半配置行)。
async fn find(ctx: &CoreContext) -> CoreResult<Option<sync_configs::Model>> {
    Ok(sync_configs::Entity::find_by_id(sync_configs::SINGLETON_ID)
        .one(&ctx.db)
        .await?)
}

/// 对外:读取配置视图(不含口令;base_url 拆回服务器地址 + 远程目录供回填)。
pub async fn get_config(ctx: &CoreContext) -> CoreResult<SyncConfigView> {
    Ok(match find(ctx).await? {
        Some(m) => {
            let (server_url, remote_dir) = split_base_url(&m.base_url);
            SyncConfigView {
                configured: true,
                server_url: Some(server_url),
                remote_dir: Some(remote_dir),
                base_url: Some(m.base_url),
                username: Some(m.username),
                password_set: m.password_ref.is_some(),
                last_backup_at: m.last_backup_at,
                last_backup_result: m.last_backup_result,
                last_backup_error: m.last_backup_error,
            }
        }
        None => SyncConfigView {
            configured: false,
            server_url: None,
            remote_dir: None,
            base_url: None,
            username: None,
            password_set: false,
            last_backup_at: None,
            last_backup_result: None,
            last_backup_error: None,
        },
    })
}

/// 保存/更新配置(upsert 单例)。口令变更经 SecretStore 存取,旧引用密文顺手清除。
pub async fn save_config(
    ctx: &CoreContext,
    input: SaveSyncConfigInput,
) -> CoreResult<SyncConfigView> {
    let server_url = normalize_server_url(&input.server_url)?;
    let remote_dir = normalize_remote_dir(input.remote_dir.as_deref())?;
    let base_url = join_base_url(&server_url, &remote_dir);
    let username = input.username.trim().to_string();
    if username.is_empty() {
        return Err(CoreError::new(
            ErrorCode::ValidationFailed,
            "用户名不能为空",
        ));
    }

    let existing = find(ctx).await?;
    // 口令:None=保留;Some("")=清除;Some(p)=重写(旧密文清理)
    let password_ref = match &input.password {
        None => existing.as_ref().and_then(|m| m.password_ref.clone()),
        Some(p) if p.is_empty() => {
            if let Some(old) = existing.as_ref().and_then(|m| m.password_ref.clone()) {
                let _ = ctx.secrets.remove(&old);
            }
            None
        }
        Some(p) => {
            let new_ref = new_id();
            ctx.secrets.store(&new_ref, p.as_bytes())?;
            if let Some(old) = existing.as_ref().and_then(|m| m.password_ref.clone()) {
                let _ = ctx.secrets.remove(&old);
            }
            Some(new_ref)
        }
    };

    let now = now_rfc3339();
    let model = sync_configs::ActiveModel {
        id: Set(sync_configs::SINGLETON_ID.to_string()),
        base_url: Set(base_url),
        username: Set(username),
        password_ref: Set(password_ref),
        last_backup_at: Set(existing.as_ref().and_then(|m| m.last_backup_at.clone())),
        last_backup_result: Set(existing.as_ref().and_then(|m| m.last_backup_result.clone())),
        last_backup_error: Set(existing.as_ref().and_then(|m| m.last_backup_error.clone())),
        updated_at: Set(now),
    };
    sync_configs::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(sync_configs::Column::Id)
                .update_columns([
                    sync_configs::Column::BaseUrl,
                    sync_configs::Column::Username,
                    sync_configs::Column::PasswordRef,
                    sync_configs::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(&ctx.db)
        .await?;
    get_config(ctx).await
}

/// 删除配置(连同已存口令密文)。
pub async fn delete_config(ctx: &CoreContext) -> CoreResult<()> {
    if let Some(m) = find(ctx).await? {
        if let Some(old) = &m.password_ref {
            let _ = ctx.secrets.remove(old);
        }
        sync_configs::Entity::delete_by_id(m.id)
            .exec(&ctx.db)
            .await?;
    }
    Ok(())
}

/// 取配置 + 解出口令,组装 WebDAV 客户端配置;未配置/缺口令 → 结构化错误。
async fn require_webdav(ctx: &CoreContext) -> CoreResult<webdav::WebDavConfig> {
    let Some(m) = find(ctx).await? else {
        return Err(CoreError::new(
            ErrorCode::SyncNotConfigured,
            "尚未配置 WebDAV 同步",
        ));
    };
    let Some(password_ref) = &m.password_ref else {
        return Err(CoreError::new(
            ErrorCode::SyncNotConfigured,
            "WebDAV 口令未设置,请先在设置页保存口令",
        ));
    };
    let password = ctx.secrets.load(password_ref)?;
    Ok(webdav::WebDavConfig {
        base_url: m.base_url,
        username: m.username,
        password: String::from_utf8_lossy(&password).to_string(),
    })
}

/// 测试连接(MKCOL 幂等确保远端目录)。
pub async fn test_connection(ctx: &CoreContext) -> CoreResult<()> {
    let cfg = require_webdav(ctx).await?;
    webdav::test_connection(&cfg).await
}

/// 列出远端备份文件(按前缀过滤,新在前)。
pub async fn list_backups(ctx: &CoreContext) -> CoreResult<Vec<RemoteBackupItem>> {
    let cfg = require_webdav(ctx).await?;
    let files = webdav::list(&cfg).await?;
    Ok(files
        .into_iter()
        .filter(|f| f.name.starts_with(BACKUP_NAME_PREFIX) && f.name.ends_with(".age"))
        .map(|f| RemoteBackupItem {
            name: f.name,
            size: f.size,
            modified: f.modified,
        })
        .collect())
}

/// 立即备份:打包(口令加密)→ 上传 → 记录结果。失败也落 last_backup_result=failed。
pub async fn backup_now(ctx: &CoreContext, passphrase: &str) -> CoreResult<RemoteBackupItem> {
    let cfg = require_webdav(ctx).await?;
    let db_path = ctx.data_dir.join("autohttps.db");
    let result = async {
        let bytes = backup::pack_backup(
            &ctx.db,
            &db_path,
            &ctx.data_dir,
            passphrase,
            &ctx.app_version,
        )
        .await?;
        let name = format!(
            "{BACKUP_NAME_PREFIX}{}.age",
            now_rfc3339()
                .replace([':', '-'], "")
                .replace('T', "-")
                .trim_end_matches('Z')
        );
        webdav::upload(&cfg, &name, bytes.clone()).await?;
        Ok(RemoteBackupItem {
            size: Some(bytes.len() as u64),
            name,
            modified: None,
        })
    }
    .await;
    record_backup_result(ctx, &result).await?;
    result
}

/// 备份结果落库(尽力而为:记录失败不覆盖业务结果)。
async fn record_backup_result(
    ctx: &CoreContext,
    result: &CoreResult<RemoteBackupItem>,
) -> CoreResult<()> {
    let Some(m) = find(ctx).await? else {
        return Ok(());
    };
    let mut am: sync_configs::ActiveModel = m.into();
    match result {
        Ok(_) => {
            am.last_backup_at = Set(Some(now_rfc3339()));
            am.last_backup_result = Set(Some("success".to_string()));
            am.last_backup_error = Set(None);
        }
        Err(e) => {
            am.last_backup_result = Set(Some("failed".to_string()));
            am.last_backup_error = Set(Some(format!("{:?}: {}", e.code, e.message)));
        }
    }
    am.updated_at = Set(now_rfc3339());
    am.update(&ctx.db).await?;
    Ok(())
}

/// 从远端恢复(在线恢复,不挪动活跃库文件 —— Windows 下活跃库被本进程占用,rename 会
/// 触发 os error 32):下载 → 解密解析(内存,任何失败都在写盘前返回)→ 用 `VACUUM INTO`
/// 从备份库**反向导入**活跃库连接(单事务,句柄不动)→ 密钥材料覆盖写 → 要求重启。
///
/// 回滚保障:恢复前的旧现场即「刚被覆盖的库」本身已随上一次备份留存于远端;
/// 且本函数先把当前库 VACUUM INTO 到 `restore-archive/pre-restore.db`(在线,不占句柄)。
///
/// 注意:写回的是另一个数据目录的密钥材料,当前进程的密钥缓存即失效,
/// 必须由宿主形态重启应用(server 重启进程 / desktop 重启 App)。
pub async fn restore(
    ctx: &CoreContext,
    remote_name: &str,
    passphrase: &str,
) -> CoreResult<RestoreOutcome> {
    // 防路径穿越/任意下载:仅允许本应用前缀的备份文件名
    if !remote_name.starts_with(BACKUP_NAME_PREFIX)
        || !remote_name.ends_with(".age")
        || remote_name.contains('/')
        || remote_name.contains('\\')
        || remote_name.contains("..")
    {
        return Err(CoreError::new(
            ErrorCode::ValidationFailed,
            "非法的备份文件名",
        ));
    }
    let cfg = require_webdav(ctx).await?;
    let encrypted = webdav::download(&cfg, remote_name).await?;

    // 解密 + 解析(内存;口令错/包损坏在此即返回,尚未写任何盘)
    let parsed = backup::parse_backup(&encrypted, passphrase)?;

    let archive_dir = ctx.data_dir.join(ARCHIVE_DIR_NAME);
    std::fs::create_dir_all(&archive_dir)
        .map_err(|e| CoreError::internal(format!("创建归档目录失败: {e}")))?;

    // 1) 备份库快照落临时文件(VACUUM INTO 需要一个磁盘上的源库)
    let incoming = archive_dir.join("incoming.db");
    std::fs::write(&incoming, &parsed.db_bytes)
        .map_err(|e| CoreError::internal(format!("暂存备份库失败: {e}")))?;

    // 2) 当前现场在线备份(不占句柄):活跃库 VACUUM INTO → pre-restore.db(可回滚)
    let pre_restore = archive_dir.join("pre-restore.db");
    let _ = std::fs::remove_file(&pre_restore);
    vacuum_into(ctx, &pre_restore).await?;

    // 3) 在线恢复:把备份库内容导入活跃库
    //    SQLite 没有直接 "import from file",用 ATTACH + 逐表复制;在事务内完成。
    restore_db_from(ctx, &incoming).await?;
    let _ = std::fs::remove_file(&incoming);

    // 4) 密钥材料覆盖写(Windows 允许覆盖写,受限的是 rename;`.age` 密文不被本进程常驻打开)
    let secrets_dir = ctx.data_dir.join("secrets");
    std::fs::create_dir_all(&secrets_dir)
        .map_err(|e| CoreError::internal(format!("创建密钥目录失败: {e}")))?;
    let mut restored = 0u32;
    for (name, bytes) in &parsed.secrets {
        std::fs::write(secrets_dir.join(name), bytes)
            .map_err(|e| CoreError::internal(format!("写入密钥材料失败: {e}")))?;
        if name != "master.key" {
            restored += 1;
        }
    }

    Ok(RestoreOutcome {
        restored_from: remote_name.to_string(),
        backup_created_at: parsed.manifest.created_at,
        secrets_restored: restored,
        requires_restart: true,
    })
}

/// 把当前活跃库在线导出到 `dest`(VACUUM INTO;不占源句柄,可在活跃连接上执行)。
async fn vacuum_into(ctx: &CoreContext, dest: &std::path::Path) -> CoreResult<()> {
    use sea_orm::ConnectionTrait;
    let dest_str = dest.to_string_lossy().replace('\\', "/");
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("VACUUM INTO '{}';", dest_str.replace('\'', "''")),
        ))
        .await?;
    Ok(())
}

/// 在线把 `src` 库文件的内容导入活跃库(ATTACH + 逐表替换,事务内)。
///
/// 备份来自同应用,schema 一致;迁移表一并替换以保持版本记录一致。
/// 对外仅经 `restore` 调用;`pub` 仅供集成测试直接驱动该核心路径(Windows 文件锁修复点)。
#[doc(hidden)]
pub async fn restore_db_from(ctx: &CoreContext, src: &std::path::Path) -> CoreResult<()> {
    use sea_orm::ConnectionTrait;
    let db = &ctx.db;
    let src_str = src.to_string_lossy().replace('\\', "/");
    let esc = src_str.replace('\'', "''");

    // 逐表复制需在事务外 ATTACH(ATTACH 可在事务内,但为清晰起见先 ATTACH 再开事务)
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        format!("ATTACH DATABASE '{esc}' AS src;"),
    ))
    .await?;

    let result = restore_tables_via_attach(db).await;

    // 无论成败都 DETACH(尽力而为)
    let _ = db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "DETACH DATABASE src;".to_string(),
        ))
        .await;
    result
}

/// ATTACH 后的事务内逐表替换(独立成函数便于确保 DETACH 总能执行)。
///
/// 按**列名交集**对齐(非 `SELECT *`):备份与活跃库 schema 可能因版本差异列不一致
/// (如恢复旧代码打的备份),`SELECT *` 会因列数不符直接报错。交集对齐容忍加列/删列;
/// 只删活跃库中备份也有的表,备份没有的表(如新版新增表)保留现值不动。
async fn restore_tables_via_attach(db: &sea_orm::DatabaseConnection) -> CoreResult<()> {
    use sea_orm::ConnectionTrait;
    // 目标表集合 = src 中的用户表(排除 sqlite_ 系统表)
    let rows = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT name FROM src.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';"
                .to_string(),
        ))
        .await
        .map_err(db_err("读取备份库表清单"))?;
    let mut tables: Vec<String> = Vec::new();
    for r in &rows {
        let name: String = r
            .try_get_by_index(0)
            .map_err(|e| CoreError::internal(format!("读取备份库表名失败: {e}")))?;
        tables.push(name);
    }

    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "BEGIN IMMEDIATE;".to_string(),
    ))
    .await
    .map_err(db_err("开启恢复事务"))?;
    for t in &tables {
        let result = restore_one_table(db, t).await;
        if let Err(e) = result {
            let _ = db
                .execute(sea_orm::Statement::from_string(
                    sea_orm::DatabaseBackend::Sqlite,
                    "ROLLBACK;".to_string(),
                ))
                .await;
            return Err(e);
        }
    }
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "COMMIT;".to_string(),
    ))
    .await
    .map_err(db_err("提交恢复事务"))?;
    Ok(())
}

/// 单表替换:取活跃库与备份库的列名交集,按交集列 DELETE + INSERT(标识符加引号)。
async fn restore_one_table(db: &sea_orm::DatabaseConnection, t: &str) -> CoreResult<()> {
    use sea_orm::ConnectionTrait;
    let main_cols = table_columns(db, "main", t).await?;
    if main_cols.is_empty() {
        // 活跃库没有这张表(备份来自更高版本)—— 跳过,不建表(避免引入未迁移 schema)
        return Ok(());
    }
    let src_cols = table_columns(db, "src", t).await?;
    let common: Vec<&String> = main_cols.iter().filter(|c| src_cols.contains(c)).collect();
    if common.is_empty() {
        return Ok(());
    }
    let cols = common
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let q = format!(
        "DELETE FROM \"{t}\"; INSERT INTO \"{t}\" ({cols}) SELECT {cols} FROM src.\"{t}\";",
    );
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        q,
    ))
    .await
    .map_err(db_err(&format!("恢复表 {t}")))?;
    Ok(())
}

/// 读某 schema 下某表的列名(PRAGMA table_info)。
async fn table_columns(
    db: &sea_orm::DatabaseConnection,
    schema: &str,
    table: &str,
) -> CoreResult<Vec<String>> {
    use sea_orm::ConnectionTrait;
    let rows = db
        .query_all(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("PRAGMA {schema}.table_info(\"{table}\");"),
        ))
        .await
        .map_err(db_err(&format!("读取表 {table} 结构")))?;
    let mut cols = Vec::new();
    for r in &rows {
        let name: String = r
            .try_get_by_index(1)
            .map_err(|e| CoreError::internal(format!("解析表 {table} 列名失败: {e}")))?;
        cols.push(name);
    }
    Ok(cols)
}

/// 把底层 DbErr 透传进恢复错误的 message(恢复是运维诊断场景,需要真因;不套统一抹平)。
fn db_err(what: &str) -> impl Fn(sea_orm::DbErr) -> CoreError + '_ {
    move |e| CoreError::internal(format!("{what}失败: {e}"))
}
