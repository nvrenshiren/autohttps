/**
 * 任务状态可用操作判定 —— **前端 UI 门控**(设计 H4)。是 core 任务状态机的 TS 镜像:后端
 * (retry/cancel)是权威强制方,前端据此禁用 / 启用 + Tooltip 原因。转移权威:flows/tasks §3。
 */
import type { TaskStatus } from "@/bindings";

/** 可重试:仅失败任务(TT7)。 */
export function canRetryTask(s: TaskStatus): boolean {
  return s === "failed";
}

/** 可取消:排队 / 执行中(TT5 / TT6);执行中为尽力而为(DT2)。 */
export function canCancelTask(s: TaskStatus): boolean {
  return s === "queued" || s === "running";
}
