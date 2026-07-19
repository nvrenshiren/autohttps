//! 服务层集成测试 —— 内存 SQLite(单连接池)+ 临时数据目录,走真实迁移建库。
//!
//! 覆盖:boot 崩溃恢复(running→failed)、证书服务源态门控(renew/revoke/retry/delete)、
//! create 全链路(证书 + SAN 关联 + 入队任务)、delete 清理(任务取消 + 密钥文件移除)。

use autohttps_core::enums::{CertificateStatus, IssuanceMethod, TaskStatus, TaskTrigger, TaskType};
use autohttps_core::persistence::entities::{certificates, domains, sync_configs, tasks};
use autohttps_core::services::certificates::{self as cert_svc, IssueCertInput};
use autohttps_core::services::{boot, tasks as task_svc};
use autohttps_core::CoreContext;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

/// 单连接内存库:max_connections=1 保证迁移与测试看到的是同一连接、同一库。
async fn mem_db() -> sea_orm::DatabaseConnection {
    let opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("建内存库失败");
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
    autohttps_core::persistence::migration::Migrator::up(&db, None)
        .await
        .expect("迁移失败");
    db
}

async fn test_ctx() -> (CoreContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("建临时目录失败");
    let db = mem_db().await;
    let ctx = CoreContext::new(
        db,
        dir.path().to_path_buf(),
        autohttps_core::enums::RunMode::Server,
        "0.0.0-test".to_string(),
    );
    (ctx, dir)
}

/// 造一个域名行,返回其 id。
async fn insert_domain(db: &sea_orm::DatabaseConnection, hostname: &str) -> String {
    let id = autohttps_core::util::new_id();
    let now = autohttps_core::util::now_rfc3339();
    domains::ActiveModel {
        id: Set(id.clone()),
        hostname: Set(hostname.to_string()),
        is_wildcard: Set(false),
        validation_method: Set(None),
        group_name: Set(None),
        remark: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("插域名失败");
    id
}

/// 造一个指定状态的自签证书行(可选根 CA 来源引用),返回其 id。
async fn insert_cert(
    db: &sea_orm::DatabaseConnection,
    status: CertificateStatus,
    with_material: bool,
) -> String {
    insert_cert_with_ca(db, status, with_material, None).await
}

/// 同 [`insert_cert`],但挂根 CA 来源(续签/重试的来源前置校验需要)。
async fn insert_cert_with_ca(
    db: &sea_orm::DatabaseConnection,
    status: CertificateStatus,
    with_material: bool,
    root_ca_id: Option<String>,
) -> String {
    let id = autohttps_core::util::new_id();
    let now = autohttps_core::util::now_rfc3339();
    certificates::ActiveModel {
        id: Set(id.clone()),
        issuance_method: Set(IssuanceMethod::SelfSigned),
        status: Set(status),
        acme_account_id: Set(None),
        root_ca_id: Set(root_ca_id),
        serial_number: Set(with_material.then(|| "01AB".to_string())),
        fingerprint: Set(None),
        not_before: Set(None),
        not_after: Set(with_material.then(|| {
            // 距到期 180 天:can_renew/can_revoke 的 valid 源态不判有效期
            (time::OffsetDateTime::now_utc() + time::Duration::days(180))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        })),
        issued_at: Set(None),
        cert_pem_ref: Set(None),
        private_key_ref: Set(None),
        last_error: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("插证书失败");
    id
}

/// 造一个指定状态的任务行,返回其 id。
async fn insert_task(
    db: &sea_orm::DatabaseConnection,
    cert_id: &str,
    task_type: TaskType,
    status: TaskStatus,
) -> String {
    let id = autohttps_core::util::new_id();
    let now = autohttps_core::util::now_rfc3339();
    tasks::ActiveModel {
        id: Set(id.clone()),
        task_type: Set(task_type),
        status: Set(status),
        trigger: Set(TaskTrigger::Manual),
        certificate_id: Set(cert_id.to_string()),
        parent_task_id: Set(None),
        attempt_number: Set(1),
        result_summary: Set(None),
        failure_reason: Set(None),
        queued_at: Set(now.clone()),
        started_at: Set(None),
        finished_at: Set(None),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("插任务失败");
    id
}

#[tokio::test]
async fn boot_recovery_marks_running_tasks_failed() {
    let (ctx, _dir) = test_ctx().await;
    let cert_id = insert_cert(&ctx.db, CertificateStatus::Issuing, false).await;
    let running = insert_task(&ctx.db, &cert_id, TaskType::Issue, TaskStatus::Running).await;
    let queued = insert_task(&ctx.db, &cert_id, TaskType::Issue, TaskStatus::Queued).await;

    let recovered = boot::recover_tasks(&ctx).await.expect("恢复失败");
    assert_eq!(recovered, 1);

    let t = tasks::Entity::find_by_id(&running)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(t.status, TaskStatus::Failed);
    assert!(t.finished_at.is_some());
    assert!(t.failure_reason.unwrap().contains("可重试"));

    // queued 保持不动(交执行器接管)
    let q = tasks::Entity::find_by_id(&queued)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(q.status, TaskStatus::Queued);
}

#[tokio::test]
async fn create_enqueues_issue_task_and_links_san() {
    let (ctx, _dir) = test_ctx().await;
    let domain_id = insert_domain(&ctx.db, "intranet.local").await;
    let ca_id = insert_active_root_ca(&ctx).await;

    let detail = cert_svc::create(
        &ctx,
        IssueCertInput {
            issuance_method: IssuanceMethod::SelfSigned,
            domain_ids: vec![domain_id],
            acme_account_id: None,
            root_ca_id: Some(ca_id),
        },
    )
    .await
    .expect("创建应成功");
    assert_eq!(detail.row.cert.status, CertificateStatus::PendingIssue);

    let queued = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(&detail.row.cert.id))
        .one(&ctx.db)
        .await
        .unwrap()
        .expect("应已入队任务");
    assert_eq!(queued.task_type, TaskType::Issue);
    assert_eq!(queued.status, TaskStatus::Queued);
}

#[tokio::test]
async fn create_rejects_missing_domain() {
    let (ctx, _dir) = test_ctx().await;
    let ca_id = insert_active_root_ca(&ctx).await;
    let res = cert_svc::create(
        &ctx,
        IssueCertInput {
            issuance_method: IssuanceMethod::SelfSigned,
            domain_ids: vec!["nonexistent".to_string()],
            acme_account_id: None,
            root_ca_id: Some(ca_id),
        },
    )
    .await;
    let err = res.err().expect("引用不存在域名应报错");
    assert_eq!(err.code, autohttps_core::ErrorCode::InvalidDomainReference);
}

#[tokio::test]
async fn renew_and_revoke_gate_on_source_state() {
    let (ctx, _dir) = test_ctx().await;

    // valid → 可续签、可吊销(挂 active 根 CA 供来源前置校验)
    let ca_id = insert_active_root_ca(&ctx).await;
    let cert_id = insert_cert_with_ca(&ctx.db, CertificateStatus::Valid, true, Some(ca_id)).await;
    cert_svc::renew(&ctx, &cert_id)
        .await
        .expect("valid 应可续签");
    let c = certificates::Entity::find_by_id(&cert_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c.status, CertificateStatus::Renewing);

    // renewing(进行中)→ 不可再续签
    let err = cert_svc::renew(&ctx, &cert_id)
        .await
        .err()
        .expect("renewing 不可续签");
    assert_eq!(err.code, autohttps_core::ErrorCode::InvalidCertState);

    // issue_failed → 不可吊销(权威转移表)
    let failed = insert_cert(&ctx.db, CertificateStatus::IssueFailed, false).await;
    let err = cert_svc::revoke(&ctx, &failed)
        .await
        .err()
        .expect("issue_failed 不可吊销");
    assert_eq!(err.code, autohttps_core::ErrorCode::InvalidCertState);
}

#[tokio::test]
async fn retry_derives_new_task_with_parent() {
    let (ctx, _dir) = test_ctx().await;
    let ca_id = insert_active_root_ca(&ctx).await;
    let cert_id =
        insert_cert_with_ca(&ctx.db, CertificateStatus::IssueFailed, false, Some(ca_id)).await;
    let failed_task = insert_task(&ctx.db, &cert_id, TaskType::Issue, TaskStatus::Failed).await;

    cert_svc::retry(&ctx, &cert_id)
        .await
        .expect("issue_failed 应可重试");

    let c = certificates::Entity::find_by_id(&cert_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        c.status,
        CertificateStatus::Issuing,
        "T5:issue_failed → issuing"
    );

    let derived = tasks::Entity::find()
        .filter(tasks::Column::CertificateId.eq(&cert_id))
        .filter(tasks::Column::Status.eq(TaskStatus::Queued))
        .one(&ctx.db)
        .await
        .unwrap()
        .expect("应派生新任务");
    assert_eq!(
        derived.parent_task_id.as_deref(),
        Some(failed_task.as_str())
    );
    assert_eq!(derived.attempt_number, 2, "attempt_number 随重试链递增");
}

#[tokio::test]
async fn delete_cancels_unfinished_tasks_and_removes_secrets() {
    let (ctx, dir) = test_ctx().await;
    let cert_id = insert_cert(&ctx.db, CertificateStatus::Valid, true).await;
    let queued = insert_task(&ctx.db, &cert_id, TaskType::Renew, TaskStatus::Queued).await;
    let done = insert_task(&ctx.db, &cert_id, TaskType::Issue, TaskStatus::Succeeded).await;

    // 造两份密钥材料(假装是 cert/key ref)
    ctx.secrets.store("deadbeef-cert", b"CERT").unwrap();
    ctx.secrets.store("deadbeef-key", b"KEY").unwrap();
    certificates::ActiveModel {
        id: Set(cert_id.clone()),
        cert_pem_ref: Set(Some("deadbeef-cert".to_string())),
        private_key_ref: Set(Some("deadbeef-key".to_string())),
        ..Default::default()
    }
    .update(&ctx.db)
    .await
    .unwrap();

    cert_svc::delete(&ctx, &cert_id).await.expect("删除应成功");

    assert!(certificates::Entity::find_by_id(&cert_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .is_none());
    let t = tasks::Entity::find_by_id(&queued)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(t.status, TaskStatus::Cancelled);
    assert_eq!(t.trigger, TaskTrigger::Cleanup);
    // 历史任务只读保留
    let h = tasks::Entity::find_by_id(&done)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(h.status, TaskStatus::Succeeded);
    // 密钥文件已移除
    assert!(!dir.path().join("secrets/deadbeef-cert.age").exists());
    assert!(!dir.path().join("secrets/deadbeef-key.age").exists());
}

#[tokio::test]
async fn delete_rejects_in_progress_cert() {
    let (ctx, _dir) = test_ctx().await;
    let cert_id = insert_cert(&ctx.db, CertificateStatus::Issuing, false).await;
    let err = cert_svc::delete(&ctx, &cert_id)
        .await
        .expect_err("进行中不可删除");
    assert_eq!(
        err.code,
        autohttps_core::ErrorCode::CertInProgressCannotDelete
    );
}

#[tokio::test]
async fn cancel_task_rolls_back_cert_state() {
    let (ctx, _dir) = test_ctx().await;
    let cert_id = insert_cert(&ctx.db, CertificateStatus::Issuing, false).await;
    let task_id = insert_task(&ctx.db, &cert_id, TaskType::Issue, TaskStatus::Running).await;

    task_svc::cancel_task(&ctx, &task_id)
        .await
        .expect("取消应成功");

    let t = tasks::Entity::find_by_id(&task_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(t.status, TaskStatus::Cancelled);
    let c = certificates::Entity::find_by_id(&cert_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    // T22:签发中取消 → issue_failed
    assert_eq!(c.status, CertificateStatus::IssueFailed);
}

/// 造一个 active 根 CA(生成真实密钥材料,供 self_signed 来源校验),返回其 id。
async fn insert_active_root_ca(ctx: &CoreContext) -> String {
    use autohttps_core::persistence::entities::root_cas;
    let key_ref = autohttps_core::util::new_id();
    ctx.secrets
        .store(&key_ref, b"ROOT-KEY-PLACEHOLDER")
        .unwrap();
    let id = autohttps_core::util::new_id();
    let now = autohttps_core::util::now_rfc3339();
    let not_after = (time::OffsetDateTime::now_utc() + time::Duration::days(3650))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    root_cas::ActiveModel {
        id: Set(id.clone()),
        name: Set("测试根 CA".to_string()),
        status: Set(autohttps_core::enums::RootCaStatus::Active),
        creation_method: Set("generated".to_string()),
        not_before: Set(now.clone()),
        not_after: Set(not_after),
        serial_number: Set(None),
        fingerprint: Set(None),
        cert_pem: Set("CERT-PLACEHOLDER".to_string()),
        private_key_ref: Set(key_ref),
        created_at: Set(now.clone()),
        updated_at: Set(now),
    }
    .insert(&ctx.db)
    .await
    .expect("插根 CA 失败");
    id
}

#[tokio::test]
async fn boot_sweeps_orphan_secret_files() {
    let (ctx, dir) = test_ctx().await;
    let secrets_dir = dir.path().join("secrets");
    std::fs::create_dir_all(&secrets_dir).unwrap();

    // 一份被引用的材料(挂在证书 cert_pem_ref 上)+ 两份孤儿 + 一个非 .age 文件
    let cert_id = insert_cert(&ctx.db, CertificateStatus::Valid, true).await;
    ctx.secrets.store("live-cert", b"CERT").unwrap();
    ctx.secrets.store("orphan-a", b"X").unwrap();
    ctx.secrets.store("orphan-b", b"Y").unwrap();
    std::fs::write(secrets_dir.join("master.key"), "AGE-SECRET-KEY-TEST").unwrap();
    certificates::ActiveModel {
        id: Set(cert_id.clone()),
        cert_pem_ref: Set(Some("live-cert".to_string())),
        ..Default::default()
    }
    .update(&ctx.db)
    .await
    .unwrap();

    let swept = boot::sweep_orphan_secrets(&ctx).await.expect("清扫失败");
    assert_eq!(swept, 2);
    assert!(secrets_dir.join("live-cert.age").exists(), "被引用材料不动");
    assert!(!secrets_dir.join("orphan-a.age").exists());
    assert!(!secrets_dir.join("orphan-b.age").exists());
    assert!(secrets_dir.join("master.key").exists(), "master.key 不动");
}

/// 回归:boot 清扫不得删掉 `sync_configs.password_ref` 引用的 WebDAV 口令密文。
/// 漏算该引用会让每次启动都把已存口令当孤儿删掉 —— 表现为「重开应用后 WebDAV 密码丢失」。
#[tokio::test]
async fn boot_sweep_keeps_sync_password_secret() {
    let (ctx, dir) = test_ctx().await;
    let secrets_dir = dir.path().join("secrets");

    ctx.secrets.store("webdav-pass", b"P4SS").unwrap();
    let now = autohttps_core::util::now_rfc3339();
    sync_configs::ActiveModel {
        id: Set(sync_configs::SINGLETON_ID.to_string()),
        base_url: Set("https://dav.example.com/autohttps/".to_string()),
        username: Set("alice".to_string()),
        password_ref: Set(Some("webdav-pass".to_string())),
        last_backup_at: Set(None),
        last_backup_result: Set(None),
        last_backup_error: Set(None),
        updated_at: Set(now),
    }
    .insert(&ctx.db)
    .await
    .expect("插同步配置失败");

    let swept = boot::sweep_orphan_secrets(&ctx).await.expect("清扫失败");
    assert_eq!(swept, 0, "被同步配置引用的口令不是孤儿");
    assert!(
        secrets_dir.join("webdav-pass.age").exists(),
        "口令密文必须保留"
    );
}
