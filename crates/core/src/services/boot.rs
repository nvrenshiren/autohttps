//! Boot 序列(ARCHITECTURE §7)—— 进程启动 → 开始服务前,由 server/desktop bin 统一调用。
//!
//! 实现:1) **任务崩溃恢复**(遗留 `running` → `failed` 可重试);2) **孤儿密钥材料清扫**
//! (多步写中途崩溃遗留的无引用 `.age` 文件);3) **启动即全量扫描**(证书 T6/T10、根 CA L3)
//! + 4) 依 settings **自动续签**(经任务队列)。执行器/扫描周期任务由 bin 在本序列之后 spawn。

use crate::domain::enums::TaskStatus;
use crate::domain::error::CoreResult;
use crate::persistence::entities::{acme_accounts, certificates, root_cas, sync_configs, tasks};
use crate::services::context::CoreContext;
use crate::util::now_rfc3339;
use sea_orm::*;
use std::collections::HashSet;

/// boot 序列第 1 步:任务崩溃恢复(tasks §3.3 底线:不卡死)。
///
/// 遗留 `running`(上一进程执行中崩溃)→ 校正 `failed`(可重试);实际结果由后续证书扫描据实校正
/// (DT2)。`queued` 保持不动(交执行器 worker 接管消费)。
pub async fn recover_tasks(ctx: &CoreContext) -> CoreResult<u64> {
    let db = &ctx.db;
    let running = tasks::Entity::find()
        .filter(tasks::Column::Status.eq(TaskStatus::Running))
        .all(db)
        .await?;
    let n = running.len() as u64;
    let now = now_rfc3339();
    for t in running {
        let mut a: tasks::ActiveModel = t.into();
        a.status = Set(TaskStatus::Failed);
        a.finished_at = Set(Some(now.clone()));
        a.failure_reason = Set(Some("进程重启:任务在途状态未知,已置为失败(可重试)".into()));
        a.result_summary = Set(Some("崩溃恢复校正".into()));
        a.updated_at = Set(now.clone());
        a.update(db).await?;
    }
    Ok(n)
}

/// 孤儿密钥材料清扫:删除 `secrets/` 下不再被任何实体 `*_ref` 引用的 `.age` 文件。
///
/// 多步写(先落密文、后写库行)中途崩溃会留下无引用的孤儿文件;boot 时对照
/// `certificates.cert_pem_ref/private_key_ref`、`acme_accounts.account_key_ref`、
/// `root_cas.private_key_ref`、`sync_configs.password_ref` 的全集清扫。
/// `master.key` 与非 `.age` 文件不动。返回清扫数。
pub async fn sweep_orphan_secrets(ctx: &CoreContext) -> CoreResult<u64> {
    let db = &ctx.db;
    let mut live: HashSet<String> = HashSet::new();
    for c in certificates::Entity::find().all(db).await? {
        live.extend(c.cert_pem_ref);
        live.extend(c.private_key_ref);
    }
    live.extend(
        acme_accounts::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .filter_map(|a| a.account_key_ref),
    );
    live.extend(
        root_cas::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .map(|ca| ca.private_key_ref),
    );
    live.extend(
        sync_configs::Entity::find()
            .all(db)
            .await?
            .into_iter()
            .filter_map(|s| s.password_ref),
    );

    let dir = ctx.data_dir.join("secrets");
    let mut removed = 0u64;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(0); // 目录不存在(尚无密钥材料)→ 无事可做
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(ref_key) = name.to_str().and_then(|n| n.strip_suffix(".age")) else {
            continue; // 非 .age 文件(含 master.key)不动
        };
        if !live.contains(ref_key) {
            match std::fs::remove_file(entry.path()) {
                Ok(()) => {
                    removed += 1;
                    tracing::warn!(ref_key, "清扫孤儿密钥材料(无实体引用)");
                }
                Err(e) => tracing::warn!(ref_key, error = %e, "孤儿密钥材料删除失败,跳过"),
            }
        }
    }
    Ok(removed)
}

/// 完整 boot 序列。返回崩溃恢复校正的任务数(供日志)。
///
/// 顺序要点(§7):先崩溃恢复(running→failed),使随后的扫描基于校正后的状态判定;再启动即全量
/// 扫描 + 自动续签(经任务队列)。此时执行器/扫描周期任务尚未 spawn(由 bin 在本序列之后启动),
/// 自动续签入队的任务会被随后 spawn 的执行器接管。孤儿密钥清扫在恢复后、扫描前执行。
pub async fn run(ctx: &CoreContext) -> CoreResult<u64> {
    let recovered = recover_tasks(ctx).await?;
    let swept = sweep_orphan_secrets(ctx).await?;
    if swept > 0 {
        tracing::info!(swept, "孤儿密钥材料清扫完成");
    }
    // 启动即检测:全量扫描(T6/T10 + L3)+ 依 settings 自动续签。
    let report = crate::scan::scan_once(ctx).await?;
    tracing::info!(?report, "boot 启动即全量扫描完成");
    Ok(recovered)
}
