//! 敏感数据静态存储(AR4 / 决策3)—— 私钥 / 账户密钥 / 根 CA 私钥**密文**落数据目录。
//!
//! 库内只存 `*_ref` 引用键;本模块据引用键读写密文文件。里程碑2 去桩:接入 **age**
//! (X25519 + ChaCha20-Poly1305,决策3)。主密钥(age 身份)落 `<data>/secrets/master.key`,
//! 严格文件权限保护(Unix 0600;Windows 依赖用户目录 ACL)。每份材料以 age 加密后落
//! `<data>/secrets/<ref>.age`,**明文绝不落盘、绝不入库/入日志**。
//!
//! 桌面可选叠加 OS keychain 加固(决策3)留待后续;当前跨形态基线为加密静态存储。

use crate::domain::error::{CoreError, CoreResult};
use age::secrecy::ExposeSecret;
use age::x25519::{Identity, Recipient};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

/// 敏感材料存储 —— 以数据目录下的 `secrets/` 子目录为根,age 加密。
#[derive(Clone)]
pub struct SecretStore {
    root: PathBuf,
    /// 主密钥(age 身份字符串 `AGE-SECRET-KEY-…`)—— 懒加载缓存,保 `new` 无副作用可克隆。
    identity: Arc<OnceLock<String>>,
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 绝不打印身份材料(AR4/L6)。
        f.debug_struct("SecretStore")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl SecretStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            root: data_dir.join("secrets"),
            identity: Arc::new(OnceLock::new()),
        }
    }

    fn master_key_path(&self) -> PathBuf {
        self.root.join("master.key")
    }

    fn path_for(&self, reference: &str) -> PathBuf {
        // reference 为 UUIDv7 派生键;不含路径分隔符,不越界。
        self.root.join(format!("{reference}.age"))
    }

    /// 取(必要时生成)主密钥身份。首次调用创建 `secrets/` 目录 + `master.key`。
    fn identity(&self) -> CoreResult<Identity> {
        if let Some(s) = self.identity.get() {
            return Identity::from_str(s)
                .map_err(|e| CoreError::internal(format!("主密钥解析失败: {e}")));
        }
        std::fs::create_dir_all(&self.root)
            .map_err(|e| CoreError::internal(format!("创建密钥目录失败: {e}")))?;
        let key_path = self.master_key_path();
        let secret = if key_path.exists() {
            std::fs::read_to_string(&key_path)
                .map_err(|e| CoreError::internal(format!("读取主密钥失败: {e}")))?
                .trim()
                .to_string()
        } else {
            let id = Identity::generate();
            let s = id.to_string().expose_secret().to_string();
            std::fs::write(&key_path, &s)
                .map_err(|e| CoreError::internal(format!("写入主密钥失败: {e}")))?;
            restrict_permissions(&key_path);
            s
        };
        // 缓存(忽略竞态:单进程,先到者胜,值一致)。
        let _ = self.identity.set(secret.clone());
        Identity::from_str(&secret).map_err(|e| CoreError::internal(format!("主密钥解析失败: {e}")))
    }

    fn recipient(&self) -> CoreResult<Recipient> {
        Ok(self.identity()?.to_public())
    }

    /// 落一份敏感材料(age 加密),返回引用键(存入实体的 `*_ref` 列)。
    pub fn store(&self, reference: &str, plaintext: &[u8]) -> CoreResult<String> {
        let recipient = self.recipient()?;
        let ciphertext = age::encrypt(&recipient, plaintext)
            .map_err(|e| CoreError::internal(format!("加密密钥材料失败: {e}")))?;
        std::fs::write(self.path_for(reference), ciphertext)
            .map_err(|e| CoreError::internal(format!("写入密钥材料失败: {e}")))?;
        Ok(reference.to_string())
    }

    /// 按引用键读回敏感材料(age 解密;导出端点使用)。
    pub fn load(&self, reference: &str) -> CoreResult<Vec<u8>> {
        let ciphertext = std::fs::read(self.path_for(reference))
            .map_err(|_| CoreError::internal("密钥材料不存在或不可读"))?;
        let identity = self.identity()?;
        age::decrypt(&identity, &ciphertext)
            .map_err(|e| CoreError::internal(format!("解密密钥材料失败: {e}")))
    }

    /// 按引用键清除敏感材料(证书/账户/根 CA 移除时)。
    pub fn remove(&self, reference: &str) -> CoreResult<()> {
        let p = self.path_for(reference);
        if p.exists() {
            std::fs::remove_file(p)
                .map_err(|e| CoreError::internal(format!("清除密钥材料失败: {e}")))?;
        }
        Ok(())
    }
}

/// 收紧主密钥文件权限(Unix 0600)。Windows 依赖用户目录 ACL(不额外处理)。
#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}
