//! Boot 序列(ARCHITECTURE §7)—— 进程启动 → 开始服务前,由 server/desktop bin 统一调用。
//!
//! 里程碑1 实现:**任务崩溃恢复**(遗留 `running` → `failed` 可重试)为真实逻辑;
//! **启动即全量扫描 / 自动续签** 依赖扫描器 + 执行器,打桩(TODO)。

use crate::domain::enums::TaskStatus;
use crate::domain::error::CoreResult;
use crate::persistence::entities::tasks;
use crate::services::context::CoreContext;
use crate::util::now_rfc3339;
use sea_orm::*;

/// boot 序列第 1 步:任务崩溃恢复(tasks §3.3 底线:不卡死)。
///
/// 遗留 `running`(上一进程执行中崩溃)→ 校正 `failed`(可重试);实际结果由后续证书扫描据实校正
/// (DT2)。`queued` 保持不动(等执行器接管;里程碑1 执行器打桩,故 queued 会静置)。
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
pub async fn run(ctx: &CoreContext) -> CoreResult<u64> {
    let recovered = recover_tasks(ctx).await?;
    // TODO(实现期):启动即全量扫描(certificates T6/T10、local-ca L3)+ 依 settings 自动续签(经任务队列)。
    Ok(recovered)
}
