//! 备份快照打包 / 解包(WebDAV 同步 §MVP)—— 一致性 SQLite 快照 + 密钥材料 → 单包,age 口令加密。
//!
//! **打包**(`pack_backup`):
//! 1. `VACUUM INTO` 导出一致性 DB 快照(WAL 下绝不直接拷库文件);
//! 2. zip:db 快照 + `secrets/`(全部 `.age` 密文 + `master.key`)+ `manifest.json`;
//! 3. 整个 zip 用**用户口令**(age passphrase / scrypt)加密 → 上传即安全(口令即最后防线)。
//!
//! **解包**(`unpack_backup`):口令解密 zip → 还原到目标数据目录(调用方负责先归档现场)。
//!
//! 安全口径:备份包含全部私钥材料(master.key 在内),口令强度 = 私钥安全;UI 强制最小长度
//! (`MIN_PASSPHRASE_LEN`)并醒目提示。日志脱敏:口令/材料内容绝不入日志。

use crate::domain::error::{CoreError, CoreResult};
use age::secrecy::SecretString;
use sea_orm::ConnectionTrait;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 口令最小长度(口令即私钥最后防线,UI/服务双层强制)。
pub const MIN_PASSPHRASE_LEN: usize = 10;
/// 备份包内 manifest 文件名。
const MANIFEST_NAME: &str = "manifest.json";
/// 备份包内 DB 快照文件名。
const DB_SNAPSHOT_NAME: &str = "autohttps.db";
/// zip 内 secrets 前缀(还原时据此前缀+文件名落回 `secrets/`)。
const SECRETS_PREFIX: &str = "secrets/";

/// 备份包清单(打包时写入,解包时校验)。
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupManifest {
    /// 清单格式版本(未来兼容用;当前恒 1)。
    pub format_version: u32,
    /// 生成时刻(RFC3339 UTC)。
    pub created_at: String,
    /// 生成方应用版本。
    pub app_version: String,
    /// 打包的 secrets 文件名列表(不含 `master.key` 与 `manifest.json` 本身)。
    pub secret_files: Vec<String>,
}

/// 用口令(age passphrase)加密一份明文。
fn passphrase_encrypt(passphrase: &str, plaintext: &[u8]) -> CoreResult<Vec<u8>> {
    let encryptor = age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_owned()));
    let mut out = Vec::new();
    let mut w = encryptor
        .wrap_output(&mut out)
        .map_err(|e| CoreError::internal(format!("备份加密初始化失败: {e}")))?;
    w.write_all(plaintext)
        .and_then(|()| w.finish())
        .map_err(|e| CoreError::internal(format!("备份加密失败: {e}")))?;
    Ok(out)
}

/// 用口令解密(age passphrase / scrypt 身份)。
fn passphrase_decrypt(passphrase: &str, ciphertext: &[u8]) -> CoreResult<Vec<u8>> {
    let decryptor = age::Decryptor::new(ciphertext).map_err(|_| {
        CoreError::new(
            crate::domain::error::ErrorCode::SyncPassphraseWrong,
            "备份解密失败(口令错误或文件损坏)",
        )
    })?;
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()));
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| {
            CoreError::new(
                crate::domain::error::ErrorCode::SyncPassphraseWrong,
                "备份解密失败(口令错误或文件损坏)",
            )
        })?;
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| CoreError::internal(format!("读取解密备份失败: {e}")))?;
    Ok(out)
}

/// 打包当前数据目录为口令加密的备份包(纯函数式:读 db_path + data_dir,不连服务)。
///
/// 返回加密后的字节流(可直接上传)。`app_version` 写入 manifest。
pub async fn pack_backup(
    db: &sea_orm::DatabaseConnection,
    db_path: &Path,
    data_dir: &Path,
    passphrase: &str,
    app_version: &str,
) -> CoreResult<Vec<u8>> {
    if passphrase.len() < MIN_PASSPHRASE_LEN {
        return Err(CoreError::new(
            crate::domain::error::ErrorCode::ValidationFailed,
            format!("口令至少 {MIN_PASSPHRASE_LEN} 位"),
        ));
    }

    // 1) 一致性快照:WAL checkpoint + VACUUM INTO(不直接拷活跃库文件)
    let snapshot =
        std::env::temp_dir().join(format!("autohttps-backup-{}.db", crate::util::new_id()));
    let snapshot_str = snapshot.to_string_lossy().replace('\\', "/");
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA wal_checkpoint(TRUNCATE);".to_string(),
    ))
    .await?;
    db.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        format!("VACUUM INTO '{}';", snapshot_str.replace('\'', "''")),
    ))
    .await?;
    let db_bytes = std::fs::read(&snapshot)
        .map_err(|e| CoreError::internal(format!("读取 DB 快照失败: {e}")))?;
    let _ = std::fs::remove_file(&snapshot);
    let _ = db_path; // 语义保留:快照由连接导出,与文件路径无关

    // 2) zip:manifest + db 快照 + secrets(密文 + master.key)
    let secrets_dir = data_dir.join("secrets");
    let mut secret_files: Vec<String> = Vec::new();
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let zerr = |e: zip::result::ZipError| CoreError::internal(format!("备份打包失败: {e}"));

    zw.start_file(DB_SNAPSHOT_NAME, opts).map_err(zerr)?;
    zw.write_all(&db_bytes).map_err(|e| zerr(e.into()))?;

    if secrets_dir.exists() {
        for entry in std::fs::read_dir(&secrets_dir)
            .map_err(|e| CoreError::internal(format!("读取密钥目录失败: {e}")))?
            .flatten()
        {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };
            if !name_str.ends_with(".age") && name_str != "master.key" {
                continue;
            }
            let bytes = std::fs::read(entry.path())
                .map_err(|e| CoreError::internal(format!("读取密钥材料失败: {e}")))?;
            zw.start_file(format!("{SECRETS_PREFIX}{name_str}"), opts)
                .map_err(zerr)?;
            zw.write_all(&bytes).map_err(|e| zerr(e.into()))?;
            if name_str != "master.key" {
                secret_files.push(name_str.to_string());
            }
        }
    }

    let manifest = BackupManifest {
        format_version: 1,
        created_at: crate::util::now_rfc3339(),
        app_version: app_version.to_string(),
        secret_files,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| CoreError::internal(format!("生成备份清单失败: {e}")))?;
    zw.start_file(MANIFEST_NAME, opts).map_err(zerr)?;
    zw.write_all(manifest_json.as_bytes())
        .map_err(|e| zerr(e.into()))?;

    let zip_bytes = zw.finish().map_err(zerr)?.into_inner();

    // 3) 整包口令加密
    passphrase_encrypt(passphrase, &zip_bytes)
}

/// 还原用结果(还原的文件统计,供摘要/测试)。
#[derive(Debug)]
pub struct RestoreReport {
    pub manifest: BackupManifest,
    pub secrets_restored: u32,
}

/// 解包口令加密的备份包,把 DB 与 secrets 写回数据目录(调用方负责先归档现场 + 停库)。
///
/// `db_path` 为目标库文件路径(通常 `<data_dir>/autohttps.db`)。
pub fn unpack_backup(
    encrypted: &[u8],
    passphrase: &str,
    data_dir: &Path,
    db_path: &Path,
) -> CoreResult<RestoreReport> {
    let zip_bytes = passphrase_decrypt(passphrase, encrypted)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
        .map_err(|e| CoreError::internal(format!("备份包损坏(非 zip): {e}")))?;

    // manifest 校验(每个借用域独立成块,避免 ZipArchive 多重可变借用)
    let manifest: BackupManifest = {
        let mut manifest_file = archive
            .by_name(MANIFEST_NAME)
            .map_err(|_| CoreError::internal("备份包缺少 manifest"))?;
        let mut manifest_json = String::new();
        manifest_file
            .read_to_string(&mut manifest_json)
            .map_err(|e| CoreError::internal(format!("读取备份清单失败: {e}")))?;
        serde_json::from_str(&manifest_json)
            .map_err(|e| CoreError::internal(format!("备份清单解析失败: {e}")))?
    };
    if manifest.format_version != 1 {
        return Err(CoreError::internal(format!(
            "不支持的备份格式版本 {}",
            manifest.format_version
        )));
    }

    // 还原 DB(写前调用方已归档;此处直接覆盖目标路径)
    let db_bytes = {
        let mut db_entry = archive
            .by_name(DB_SNAPSHOT_NAME)
            .map_err(|_| CoreError::internal("备份包缺少数据库快照"))?;
        let mut db_bytes = Vec::new();
        db_entry
            .read_to_end(&mut db_bytes)
            .map_err(|e| CoreError::internal(format!("读取数据库快照失败: {e}")))?;
        db_bytes
    };
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CoreError::internal(format!("创建数据目录失败: {e}")))?;
    }
    std::fs::write(db_path, &db_bytes)
        .map_err(|e| CoreError::internal(format!("写入数据库失败: {e}")))?;

    // 还原 secrets:先收集合法条目名,再逐条借用读取(防路径穿越:只取文件名部分)
    let secrets_dir: PathBuf = data_dir.join("secrets");
    std::fs::create_dir_all(&secrets_dir)
        .map_err(|e| CoreError::internal(format!("创建密钥目录失败: {e}")))?;
    let mut secret_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| CoreError::internal(format!("读取备份条目失败: {e}")))?;
        let name = entry.name().to_string();
        let Some(rest) = name.strip_prefix(SECRETS_PREFIX) else {
            continue;
        };
        // 防路径穿越:拒绝含路径分隔/上跳的条目名
        if rest.is_empty() || rest.contains('/') || rest.contains('\\') || rest.contains("..") {
            continue;
        }
        secret_names.push(rest.to_string());
    }
    let mut restored = 0u32;
    for rest in secret_names {
        let mut entry = archive
            .by_name(&format!("{SECRETS_PREFIX}{rest}"))
            .map_err(|e| CoreError::internal(format!("读取密钥条目失败: {e}")))?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| CoreError::internal(format!("读取密钥条目失败: {e}")))?;
        std::fs::write(secrets_dir.join(&rest), &bytes)
            .map_err(|e| CoreError::internal(format!("写入密钥材料失败: {e}")))?;
        if rest != "master.key" {
            restored += 1;
        }
    }

    Ok(RestoreReport {
        manifest,
        secrets_restored: restored,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrase_roundtrip() {
        let data = b"secret backup payload";
        let enc = passphrase_encrypt("correct horse battery", data).unwrap();
        assert_ne!(enc, data);
        let dec = passphrase_decrypt("correct horse battery", &enc).unwrap();
        assert_eq!(dec, data);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let enc = passphrase_encrypt("right-pass", b"x").unwrap();
        let err = passphrase_decrypt("wrong-pass", &enc).unwrap_err();
        assert_eq!(
            err.code,
            crate::domain::error::ErrorCode::SyncPassphraseWrong
        );
    }
}
