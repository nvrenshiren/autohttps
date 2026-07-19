//! WebDAV 备份同步集成测试 —— 打包/解包全链路(真实迁移建库 + 真实 SecretStore)。
//!
//! 覆盖:配置保存(口令只经 SecretStore、库内仅存 ref)、pack→unpack 往返(库行与密钥材料复原)、
//! 错误口令拒绝、非法文件名拒绝、未配置时的结构化错误。WebDAV 网络层不在此覆盖(单测已验解析)。

use autohttps_core::persistence::entities::{domains, sync_configs};
use autohttps_core::services::sync::{self as sync_svc, SaveSyncConfigInput};
use autohttps_core::sync::backup;
use autohttps_core::CoreContext;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use sea_orm_migration::MigratorTrait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

/// 文件库(非内存):pack 的 `VACUUM INTO` 在内存库上目标文件行为不可靠,文件库更贴近真实。
/// **多连接池(=4,贴近生产 8)**:在线恢复的 ATTACH 单连接修复点只有在多连接池下才会暴露
/// (单连接池会掩盖 ATTACH/查询落到不同连接的问题)。
async fn file_db(dir: &std::path::Path) -> sea_orm::DatabaseConnection {
    let opts = SqliteConnectOptions::new()
        .filename(dir.join("autohttps.db"))
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .expect("建库失败");
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
    autohttps_core::persistence::migration::Migrator::up(&db, None)
        .await
        .expect("迁移失败");
    db
}

async fn test_ctx() -> (CoreContext, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("建临时目录失败");
    let db = file_db(dir.path()).await;
    let ctx = CoreContext::new(
        db,
        dir.path().to_path_buf(),
        autohttps_core::enums::RunMode::Server,
        "0.0.0-test".to_string(),
    );
    (ctx, dir)
}

async fn insert_domain(db: &sea_orm::DatabaseConnection, hostname: &str) {
    let now = autohttps_core::util::now_rfc3339();
    domains::ActiveModel {
        id: Set(autohttps_core::util::new_id()),
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
}

#[tokio::test]
async fn config_save_keeps_password_out_of_db() {
    let (ctx, _dir) = test_ctx().await;
    let view = sync_svc::get_config(&ctx).await.expect("读配置");
    assert!(!view.configured, "初始应未配置");

    let view = sync_svc::save_config(
        &ctx,
        SaveSyncConfigInput {
            server_url: "https://dav.example.com/dav".to_string(),
            remote_dir: Some("autohttps".to_string()),
            username: "alice".to_string(),
            password: Some("webdav-secret".to_string()),
        },
    )
    .await
    .expect("存配置");
    assert!(view.configured && view.password_set);
    assert_eq!(
        view.base_url.as_deref(),
        Some("https://dav.example.com/dav/autohttps/")
    );
    assert_eq!(
        view.server_url.as_deref(),
        Some("https://dav.example.com/dav")
    );
    assert_eq!(view.remote_dir.as_deref(), Some("autohttps"));

    // 库内只有 ref,不出现口令本体
    let row = sync_configs::Entity::find_by_id("webdav")
        .one(&ctx.db)
        .await
        .expect("查行")
        .expect("行存在");
    let password_ref = row.password_ref.expect("应有口令 ref");
    assert_ne!(password_ref, "webdav-secret");
    let plain = ctx.secrets.load(&password_ref).expect("ref 可解密");
    assert_eq!(plain, b"webdav-secret");

    // 更新时口令缺省 = 保留;远程目录缺省 = 默认 autohttps
    let view = sync_svc::save_config(
        &ctx,
        SaveSyncConfigInput {
            server_url: "https://dav.example.com".to_string(),
            remote_dir: None,
            username: "alice".to_string(),
            password: None,
        },
    )
    .await
    .expect("改配置");
    assert!(view.password_set, "缺省口令应保留");
    assert_eq!(
        view.base_url.as_deref(),
        Some("https://dav.example.com/autohttps/"),
        "缺省远程目录应为 autohttps"
    );

    // 清除配置连同口令密文
    sync_svc::delete_config(&ctx).await.expect("删配置");
    let gone = ctx.secrets.load(&password_ref);
    assert!(gone.is_err(), "删除后口令密文应不可读");
}

#[tokio::test]
async fn backup_pack_unpack_roundtrip_restores_db_and_secrets() {
    let (ctx, dir) = test_ctx().await;
    insert_domain(&ctx.db, "example.com").await;
    // 造一份真实密钥材料(模拟已有证书私钥)
    let key_ref = autohttps_core::util::new_id();
    ctx.secrets
        .store(&key_ref, b"PRIVATE KEY BYTES")
        .expect("存密钥");

    let db_path = dir.path().join("autohttps.db");
    let encrypted = backup::pack_backup(
        &ctx.db,
        &db_path,
        dir.path(),
        "backup-passphrase",
        "0.0.0-test",
    )
    .await
    .expect("打包失败");

    // 解到另一个数据目录,验证内容复原
    let restore_dir = tempfile::tempdir().expect("建恢复目录");
    let report = backup::unpack_backup(
        &encrypted,
        "backup-passphrase",
        restore_dir.path(),
        &restore_dir.path().join("autohttps.db"),
    )
    .expect("解包失败");
    assert_eq!(report.manifest.format_version, 1);
    assert!(report.secrets_restored >= 1, "至少还原一份密钥材料");

    // 还原库能打开且行还在
    let db2 = file_db(restore_dir.path()).await; // 同路径(autohttps.db),迁移幂等
    let count = domains::Entity::find()
        .all(&db2)
        .await
        .expect("查域名")
        .len();
    assert_eq!(count, 1, "还原库应含备份时的域名行");

    // 还原的密钥材料可用原 master.key 解出
    let secrets2 = autohttps_core::secrets::SecretStore::new(restore_dir.path());
    let plain = secrets2.load(&key_ref).expect("还原密钥可解密");
    assert_eq!(plain, b"PRIVATE KEY BYTES");
}

/// 在线恢复(Windows 文件锁修复的核心路径):活跃库连接仍开着时,直接 ATTACH 备份库逐表替换,
/// 不 rename 库文件。验证替换后内容来自备份、且连接仍可用。
#[tokio::test]
async fn online_restore_replaces_live_db_without_rename() {
    let (ctx, dir) = test_ctx().await;
    insert_domain(&ctx.db, "live-only.com").await;
    let key_ref = autohttps_core::util::new_id();
    ctx.secrets.store(&key_ref, b"LIVE KEY").expect("存密钥");

    // 备份(此刻库里有 live-only.com)
    let db_path = dir.path().join("autohttps.db");
    let encrypted = backup::pack_backup(&ctx.db, &db_path, dir.path(), "online-restore-pass", "t")
        .await
        .expect("打包");

    // 备份后改动现场:加一行(应被恢复覆盖回只剩 live-only.com)
    insert_domain(&ctx.db, "post-backup.com").await;

    // 走服务层真实在线恢复核心:parse(内存)+ restore_db_from(ATTACH 逐表替换)
    let parsed = backup::parse_backup(&encrypted, "online-restore-pass").expect("解析");
    let incoming = dir.path().join("restore-archive").join("incoming.db");
    std::fs::create_dir_all(incoming.parent().unwrap()).unwrap();
    std::fs::write(&incoming, &parsed.db_bytes).unwrap();
    sync_svc::restore_db_from(&ctx, &incoming)
        .await
        .expect("在线恢复");

    // 活跃连接仍在用,恢复后只剩备份时的那一行
    let hosts: Vec<String> = domains::Entity::find()
        .all(&ctx.db)
        .await
        .expect("查域名")
        .into_iter()
        .map(|d| d.hostname)
        .collect();
    assert_eq!(
        hosts,
        vec!["live-only.com".to_string()],
        "应回到备份时刻的内容"
    );
}

/// 跨 schema 恢复(列交集对齐的修复点):备份库比活跃库**多一列/多一张表**时,
/// 恢复应按共有列对齐、跳过活跃库没有的表,而非因 `SELECT *` 列数不符报错。
#[tokio::test]
async fn restore_tolerates_wider_backup_schema() {
    use sea_orm::ConnectionTrait;
    let (ctx, dir) = test_ctx().await;
    insert_domain(&ctx.db, "narrow.com").await;

    // 造一个"更宽"的备份库:domains 多一列 extra_col、多一张 future_table
    let wide = dir.path().join("wide.db");
    let wdb = file_db(dir.path()).await; // 先建全量当前 schema(autohttps.db)
    drop(wdb);
    // 直接在 autohttps.db 上加列/加表,使它成为"未来版本"的库
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "ALTER TABLE domains ADD COLUMN extra_col TEXT;".to_string(),
        ))
        .await
        .expect("加列");
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "CREATE TABLE future_table(id TEXT PRIMARY KEY, v TEXT);".to_string(),
        ))
        .await
        .expect("加表");
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "INSERT INTO future_table VALUES('f1','future');".to_string(),
        ))
        .await
        .expect("插 future 行");
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!(
                "VACUUM INTO '{}';",
                wide.to_string_lossy().replace('\\', "/")
            ),
        ))
        .await
        .expect("导出 wide 备份");
    // 回滚活跃库到"当前"schema:去掉加的列/表(模拟恢复旧版本应用)
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "DROP TABLE future_table;".to_string(),
        ))
        .await
        .expect("drop future");
    ctx.db
        .execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "ALTER TABLE domains DROP COLUMN extra_col;".to_string(),
        ))
        .await
        .expect("drop extra_col");

    // 用"更宽"的备份库做在线恢复:应成功(列交集对齐 + 跳过 future_table)
    sync_svc::restore_db_from(&ctx, &wide)
        .await
        .expect("宽 schema 备份应可恢复");
    let hosts: Vec<String> = domains::Entity::find()
        .all(&ctx.db)
        .await
        .expect("查域名")
        .into_iter()
        .map(|d| d.hostname)
        .collect();
    assert_eq!(
        hosts,
        vec!["narrow.com".to_string()],
        "共有列数据应正确还原"
    );
}

/// 密钥一致性(密码丢失修复的核心):备份带来的密文用**备份里的 master.key** 加密;
/// 恢复全量替换 secrets + 清身份缓存后,备份里的密码应仍可解密(即恢复后密码不丢)。
/// 同时验证:备份里没有的旧密文被清除(不残留孤儿/悬空 ref)。
#[tokio::test]
async fn restore_keeps_backup_secrets_decryptable_and_prunes_orphans() {
    let (ctx, dir) = test_ctx().await;
    // 备份时刻:存一份密码(用当时的 master.key 加密)
    let backup_ref = autohttps_core::util::new_id();
    ctx.secrets
        .store(&backup_ref, b"WEBDAV PASSWORD")
        .expect("存备份密码");
    let db_path = dir.path().join("autohttps.db");
    let encrypted = backup::pack_backup(&ctx.db, &db_path, dir.path(), "keep-secrets-pass", "t")
        .await
        .expect("打包");

    // 备份后:master.key 轮换(模拟另一台机器/另一时刻)+ 多一份备份里没有的孤儿密文
    let master_path = dir.path().join("secrets").join("master.key");
    let backup_master = std::fs::read_to_string(&master_path).expect("读备份 master.key");
    // 生成新身份顶掉磁盘 master.key(模拟"当前进程/机器"与备份不同 key)
    std::fs::remove_file(&master_path).expect("删 master.key");
    let orphan_ref = autohttps_core::util::new_id();
    ctx.secrets.invalidate_identity_cache(); // 强制重读(会生成新 key)
    ctx.secrets
        .store(&orphan_ref, b"ORPHAN NOT IN BACKUP")
        .expect("存孤儿密文");
    let new_master = std::fs::read_to_string(&master_path).expect("读新 master.key");
    assert_ne!(backup_master, new_master, "应已轮换 master.key");

    // 恢复:parse + 全量替换 secrets(含 master.key 回滚到备份的)+ 清缓存
    let parsed = backup::parse_backup(&encrypted, "keep-secrets-pass").expect("解析");
    let secrets_dir = dir.path().join("secrets");
    let incoming: std::collections::HashSet<&str> =
        parsed.secrets.iter().map(|(n, _)| n.as_str()).collect();
    for entry in std::fs::read_dir(&secrets_dir).unwrap().flatten() {
        let n = entry.file_name();
        let ns = n.to_str().unwrap().to_string();
        if ns.ends_with(".age") && !incoming.contains(ns.as_str()) {
            std::fs::remove_file(entry.path()).unwrap();
        }
    }
    for (name, bytes) in &parsed.secrets {
        std::fs::write(secrets_dir.join(name), bytes).unwrap();
    }
    ctx.secrets.invalidate_identity_cache(); // 清缓存:磁盘 master.key 已换回备份的

    // 恢复+清缓存后:备份密码应可用备份的 master.key 解开(密码不丢)
    let recovered = ctx.secrets.load(&backup_ref).expect("备份密码应仍可解密");
    assert_eq!(recovered, b"WEBDAV PASSWORD");
    // 孤儿密文已被清除(不在备份里)
    assert!(
        !secrets_dir.join(format!("{orphan_ref}.age")).exists(),
        "备份之外的孤儿密文应被清除"
    );
    // master.key 已回滚到备份的
    assert_eq!(
        std::fs::read_to_string(&master_path).unwrap(),
        backup_master,
        "master.key 应回滚到备份版本"
    );
}

#[tokio::test]
async fn backup_rejects_short_and_wrong_passphrase() {
    let (ctx, dir) = test_ctx().await;
    let db_path = dir.path().join("autohttps.db");
    let short = backup::pack_backup(&ctx.db, &db_path, dir.path(), "short", "t").await;
    assert!(short.is_err(), "短口令应被拒");

    let encrypted = backup::pack_backup(&ctx.db, &db_path, dir.path(), "correct-passphrase", "t")
        .await
        .expect("打包");
    let err = backup::unpack_backup(&encrypted, "wrong-passphrase", dir.path(), &db_path)
        .expect_err("错口令应失败");
    assert_eq!(err.code, autohttps_core::ErrorCode::SyncPassphraseWrong);
}

#[tokio::test]
async fn actions_require_config_and_valid_filename() {
    let (ctx, _dir) = test_ctx().await;
    let err = sync_svc::test_connection(&ctx)
        .await
        .expect_err("未配置应拒绝");
    assert_eq!(err.code, autohttps_core::ErrorCode::SyncNotConfigured);

    let err = sync_svc::restore(&ctx, "../evil.age", "whatever-pass")
        .await
        .expect_err("路径穿越文件名应拒绝");
    assert_eq!(err.code, autohttps_core::ErrorCode::ValidationFailed);
    let err = sync_svc::restore(&ctx, "other-file.txt", "whatever-pass")
        .await
        .expect_err("非备份文件名应拒绝");
    assert_eq!(err.code, autohttps_core::ErrorCode::ValidationFailed);
}
