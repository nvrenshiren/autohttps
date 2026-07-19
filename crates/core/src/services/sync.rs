//! WebDAV 备份同步服务 —— 配置持久化(口令经 SecretStore)+ 备份/恢复/列远端编排。
//!
//! - 配置单例(`sync_configs` id='webdav');口令只经 `password` 入参写入,读取永远不回传;
//! - 备份 = `sync::backup::pack_backup` 打包加密 → `webdav::upload`,结果落 last_backup_*;
//! - 恢复 = `webdav::download` → **先归档现场** → `unpack_backup` 写回,返回 `requires_restart`。
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
pub struct SyncConfigView {
    pub configured: bool,
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
    pub base_url: String,
    pub username: String,
    pub password: Option<String>,
}

/// 远端备份文件项(展示用)。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RemoteBackupItem {
    pub name: String,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

/// 恢复结果(写回成功后进程需重启以重连 DB/密钥缓存)。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RestoreOutcome {
    pub restored_from: String,
    pub backup_created_at: String,
    pub secrets_restored: u32,
    pub requires_restart: bool,
}

/// 校验并归一 base_url:必须 http(s),剥末尾斜杠后再补一个(webdav 层拼名用)。
fn normalize_url(raw: &str) -> CoreResult<String> {
    let url = raw.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(CoreError::new(
            ErrorCode::ValidationFailed,
            "WebDAV 地址须以 http:// 或 https:// 开头",
        ));
    }
    Ok(format!("{}/", url.trim_end_matches('/')))
}

/// 读配置单例;未配置返回 None(读路径不隐式建行,避免半配置行)。
async fn find(ctx: &CoreContext) -> CoreResult<Option<sync_configs::Model>> {
    Ok(sync_configs::Entity::find_by_id(sync_configs::SINGLETON_ID)
        .one(&ctx.db)
        .await?)
}

/// 对外:读取配置视图(不含口令)。
pub async fn get_config(ctx: &CoreContext) -> CoreResult<SyncConfigView> {
    Ok(match find(ctx).await? {
        Some(m) => SyncConfigView {
            configured: true,
            base_url: Some(m.base_url),
            username: Some(m.username),
            password_set: m.password_ref.is_some(),
            last_backup_at: m.last_backup_at,
            last_backup_result: m.last_backup_result,
            last_backup_error: m.last_backup_error,
        },
        None => SyncConfigView {
            configured: false,
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
    let base_url = normalize_url(&input.base_url)?;
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

/// 从远端恢复:下载 → **归档现场**(db + secrets 挪到 `restore-archive/`)→ 写回 → 要求重启。
///
/// 注意:恢复写回的是另一个数据目录的 DB 与密钥材料,当前进程的 DB 连接与密钥缓存即失效,
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

    let db_path = ctx.data_dir.join("autohttps.db");
    let secrets_dir = ctx.data_dir.join("secrets");
    let archive_dir = ctx.data_dir.join(ARCHIVE_DIR_NAME);

    // 归档现场(若归档目录已存在先清掉——上次恢复留下的旧现场)
    if archive_dir.exists() {
        std::fs::remove_dir_all(&archive_dir)
            .map_err(|e| CoreError::internal(format!("清理旧归档目录失败: {e}")))?;
    }
    std::fs::create_dir_all(&archive_dir)
        .map_err(|e| CoreError::internal(format!("创建归档目录失败: {e}")))?;
    if db_path.exists() {
        std::fs::rename(&db_path, archive_dir.join("autohttps.db"))
            .map_err(|e| CoreError::internal(format!("归档当前数据库失败: {e}")))?;
    }
    if secrets_dir.exists() {
        std::fs::rename(&secrets_dir, archive_dir.join("secrets"))
            .map_err(|e| CoreError::internal(format!("归档当前密钥目录失败: {e}")))?;
    }

    // 写回(失败时尝试把现场挪回来,尽力而为)
    let report = match backup::unpack_backup(&encrypted, passphrase, &ctx.data_dir, &db_path) {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_file(&db_path);
            let _ = std::fs::remove_dir_all(&secrets_dir);
            let _ = std::fs::rename(archive_dir.join("autohttps.db"), &db_path);
            let _ = std::fs::rename(archive_dir.join("secrets"), &secrets_dir);
            let _ = std::fs::remove_dir_all(&archive_dir);
            return Err(e);
        }
    };

    Ok(RestoreOutcome {
        restored_from: remote_name.to_string(),
        backup_created_at: report.manifest.created_at,
        secrets_restored: report.secrets_restored,
        requires_restart: true,
    })
}
