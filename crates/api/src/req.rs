//! 请求体 DTO(camelCase)+ 查询参数结构。PATCH 用 double-option 区分 null(清空)与缺省(不改)。

use crate::serde_helpers::double_option;
use autohttps_core::enums::{IssuanceMethod, ValidationMethod};
use serde::Deserialize;

// ============ 请求体 ============

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCertificateRequest {
    pub issuance_method: IssuanceMethod,
    pub domain_ids: Vec<String>,
    #[serde(default)]
    pub acme_account_id: Option<String>,
    #[serde(default)]
    pub root_ca_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainRequest {
    pub hostname: String,
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
    pub remark: Option<String>,
    #[serde(default)]
    pub validation_method: Option<ValidationMethod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub group_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub remark: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub validation_method: Option<Option<ValidationMethod>>,
    /// 契约禁改(DECD2);出现即 422 hostname_immutable。
    #[serde(default)]
    pub hostname: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettingsRequest {
    #[serde(default)]
    pub renewal_advance_days: Option<i32>,
    #[serde(default)]
    pub auto_renew_enabled: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub default_acme_account_id: Option<Option<String>>,
    #[serde(default)]
    pub autostart_enabled: Option<bool>,
    #[serde(default)]
    pub listen_address: Option<String>,
    #[serde(default)]
    pub listen_port: Option<i32>,
    /// 只读(SF5);出现即 422 storage_path_read_only。
    #[serde(default)]
    pub data_storage_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutHttp01ConfigRequest {
    pub webroot_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAcmeAccountRequest {
    pub directory_url: String,
    #[serde(default)]
    pub ca_label: Option<String>,
    pub contact_email: String,
    /// 须为 true(AT1 前提);缺省 false → 422 tos_not_agreed。
    #[serde(default)]
    pub tos_agreed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchAcmeAccountRequest {
    pub contact_email: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRootCaRequest {
    pub name: String,
    /// 有效期(自 now 天数);服务层算 notBefore/notAfter。
    pub validity_days: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRootCaRequest {
    pub name: String,
    pub cert_pem: String,
    pub private_key_pem: String,
    /// 私钥受口令保护时提供(MVP 未支持加密私钥)。
    #[serde(default)]
    pub key_passphrase: Option<String>,
}

// ============ 查询参数 ============

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 证书状态,可多值(逗号分隔)。
    pub status: Option<String>,
    pub issuance_method: Option<String>,
    pub domain: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DomainListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub group: Option<String>,
    pub certificate_state: Option<String>,
    pub hostname: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub task_type: Option<String>,
    /// 任务状态,可多值(逗号分隔);`queued,running` 即队列。
    pub status: Option<String>,
    pub certificate_id: Option<String>,
    pub trigger: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogsQuery {
    pub after_seq: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExportQuery {
    /// 逗号分隔:leaf|chain|fullchain|private_key;默认 fullchain(§2.8)。
    pub parts: Option<String>,
    /// MVP 仅 pem。
    pub format: Option<String>,
    /// 含 private_key 时须为 true,否则 422 key_export_not_acknowledged。
    pub acknowledge_key_export: Option<bool>,
    /// 部署目标:nginx|apache|iis|haproxy;给出时按目标打包 zip(全部含私钥)。
    pub target: Option<String>,
    /// target=iis 时必填:PFX 加密口令。
    pub pfx_password: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub task_id: Option<String>,
    pub domain_id: Option<String>,
    pub status: Option<String>,
    pub certificate_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RootCaListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

// ============ sync(WebDAV 备份)============

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutSyncConfigRequest {
    /// 服务器地址(如 `https://dav.example.com/dav`,不含备份目录)。
    pub server_url: String,
    /// 远程目录(缺省 `autohttps`;备份与其他项目隔离)。
    #[serde(default)]
    pub remote_dir: Option<String>,
    pub username: String,
    /// 口令:缺省 = 保留已存;空串 = 清除;非空 = 重写。读取永不回传。
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupNowRequest {
    /// 备份加密口令(最少 10 位;口令即私钥最后防线)。
    pub passphrase: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    /// 远端备份文件名(来自 GET /sync/backups 列表)。
    pub remote_name: String,
    /// 备份时的加密口令。
    pub passphrase: String,
}
