//! Boot 序列(ARCHITECTURE §7)—— 进程启动 → 开始服务前,由 server/desktop bin 统一调用。
//!
//! 实现:1) **任务崩溃恢复**(遗留 `running` → `failed` 可重试);2) **启动即全量扫描**
//! (证书 T6/T10、根 CA L3)+ 3) 依 settings **自动续签**(经任务队列)。执行器/扫描周期任务
//! 由 bin 在本序列之后 spawn。

use crate::domain::enums::TaskStatus;
use crate::domain::error::CoreResult;
use crate::persistence::entities::tasks;
use crate::services::context::CoreContext;
use crate::util::now_rfc3339;
use sea_orm::*;

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

/// 完整 boot 序列。返回崩溃恢复校正的任务数(供日志)。
///
/// 顺序要点(§7):先崩溃恢复(running→failed),使随后的扫描基于校正后的状态判定;再启动即全量
/// 扫描 + 自动续签(经任务队列)。此时执行器/扫描周期任务尚未 spawn(由 bin 在本序列之后启动),
/// 自动续签入队的任务会被随后 spawn 的执行器接管。
pub async fn run(ctx: &CoreContext) -> CoreResult<u64> {
    let recovered = recover_tasks(ctx).await?;
    // 启动即检测:全量扫描(T6/T10 + L3)+ 依 settings 自动续签。
    let report = crate::scan::scan_once(ctx).await?;
    tracing::info!(?report, "boot 启动即全量扫描完成");
    Ok(recovered)
}
