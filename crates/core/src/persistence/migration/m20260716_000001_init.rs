//! 初始迁移 —— 建 11 表(DB `_overview` §2)。字段/FK/索引严格照各模块 DB 文档。
//!
//! 语句逐条执行(sqlite 驱动对多语句串行支持有别,拆开最稳)。FK 由连接层 `PRAGMA
//! foreign_keys=ON` 生效(见 `db.rs`)。时间列 TEXT·RFC3339;布尔 INTEGER 0/1;敏感只存 `*_ref`。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 建表顺序:被引用表在前(satisfy FK)。软引用列(tasks.certificate_id /
/// challenges.domain_id / internal_cert_revocations.certificate_id)不建 DB 外键。
const SCHEMA: &str = r#"
CREATE TABLE domains (
  id TEXT PRIMARY KEY NOT NULL,
  hostname TEXT NOT NULL UNIQUE,
  is_wildcard INTEGER NOT NULL,
  validation_method TEXT,
  group_name TEXT,
  remark TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_domain_group ON domains(group_name);

CREATE TABLE acme_accounts (
  id TEXT PRIMARY KEY NOT NULL,
  directory_url TEXT NOT NULL,
  ca_label TEXT,
  environment TEXT,
  contact_email TEXT NOT NULL,
  tos_agreed INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,
  ca_account_url TEXT,
  account_key_ref TEXT,
  registered_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_acme_account_status ON acme_accounts(status);
CREATE INDEX idx_acme_account_directory ON acme_accounts(directory_url);

CREATE TABLE root_cas (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  creation_method TEXT NOT NULL,
  not_before TEXT NOT NULL,
  not_after TEXT NOT NULL,
  serial_number TEXT,
  fingerprint TEXT,
  cert_pem TEXT NOT NULL,
  private_key_ref TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_rootca_status ON root_cas(status);
CREATE INDEX idx_rootca_not_after ON root_cas(not_after);

CREATE TABLE settings (
  id TEXT PRIMARY KEY NOT NULL DEFAULT 'global' CHECK (id = 'global'),
  renewal_advance_days INTEGER NOT NULL DEFAULT 30,
  auto_renew_enabled INTEGER NOT NULL DEFAULT 1,
  default_acme_account_id TEXT REFERENCES acme_accounts(id) ON DELETE SET NULL,
  autostart_enabled INTEGER,
  listen_address TEXT,
  listen_port INTEGER,
  data_storage_path TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE certificates (
  id TEXT PRIMARY KEY NOT NULL,
  issuance_method TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending_issue',
  acme_account_id TEXT REFERENCES acme_accounts(id) ON DELETE SET NULL,
  root_ca_id TEXT REFERENCES root_cas(id) ON DELETE RESTRICT,
  serial_number TEXT,
  fingerprint TEXT,
  not_before TEXT,
  not_after TEXT,
  issued_at TEXT,
  cert_pem_ref TEXT,
  private_key_ref TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_cert_status ON certificates(status);
CREATE INDEX idx_cert_issuance_method ON certificates(issuance_method);
CREATE INDEX idx_cert_not_after ON certificates(not_after);
CREATE INDEX idx_cert_acme_account ON certificates(acme_account_id);
CREATE INDEX idx_cert_root_ca ON certificates(root_ca_id);

CREATE TABLE certificate_domains (
  certificate_id TEXT NOT NULL REFERENCES certificates(id) ON DELETE CASCADE,
  domain_id TEXT NOT NULL REFERENCES domains(id) ON DELETE RESTRICT,
  PRIMARY KEY (certificate_id, domain_id)
);
CREATE INDEX idx_certdom_domain ON certificate_domains(domain_id);

CREATE TABLE http01_validation_configs (
  domain_id TEXT PRIMARY KEY NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
  webroot_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE tasks (
  id TEXT PRIMARY KEY NOT NULL,
  certificate_id TEXT NOT NULL,
  task_type TEXT NOT NULL,
  trigger TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued',
  attempt_number INTEGER NOT NULL DEFAULT 1,
  parent_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
  queued_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  result_summary TEXT,
  failure_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_task_certificate ON tasks(certificate_id);
CREATE INDEX idx_task_status ON tasks(status);
CREATE INDEX idx_task_type ON tasks(task_type);
CREATE INDEX idx_task_parent ON tasks(parent_task_id);
CREATE INDEX idx_task_queued_at ON tasks(queued_at);

CREATE TABLE task_log_entries (
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  logged_at TEXT NOT NULL,
  level TEXT NOT NULL DEFAULT 'info',
  message TEXT NOT NULL
);
CREATE INDEX idx_tasklog_task ON task_log_entries(task_id, seq);

CREATE TABLE challenges (
  id TEXT PRIMARY KEY NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  domain_id TEXT NOT NULL,
  validation_method TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  dns_txt_name TEXT,
  dns_txt_value TEXT,
  http_file_path TEXT,
  http_file_content TEXT,
  authorization_url TEXT,
  failed_reason TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_challenge_task ON challenges(task_id);
CREATE INDEX idx_challenge_status ON challenges(status);
CREATE INDEX idx_challenge_domain ON challenges(domain_id);

CREATE TABLE internal_cert_revocations (
  id TEXT PRIMARY KEY NOT NULL,
  root_ca_id TEXT NOT NULL REFERENCES root_cas(id) ON DELETE RESTRICT,
  serial_number TEXT NOT NULL,
  certificate_id TEXT,
  revoked_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE (root_ca_id, serial_number)
);
CREATE INDEX idx_revoke_rootca ON internal_cert_revocations(root_ca_id);
CREATE INDEX idx_revoke_cert ON internal_cert_revocations(certificate_id);
"#;

const DROP_TABLES: &[&str] = &[
    "internal_cert_revocations",
    "challenges",
    "task_log_entries",
    "tasks",
    "http01_validation_configs",
    "certificate_domains",
    "certificates",
    "settings",
    "root_cas",
    "acme_accounts",
    "domains",
];

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
        let db = manager.get_connection();
        for t in DROP_TABLES {
            db.execute_unprepared(&format!("DROP TABLE IF EXISTS {t}"))
                .await?;
        }
        Ok(())
    }
}
