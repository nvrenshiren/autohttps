/** 分类标签(§3.5)—— 中性 outline,不占语义色(防彩虹汤)。 */
import { KeyRound, Landmark } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import type { IssuanceMethod, TaskTrigger, TaskType, ValidationMethod } from "@/bindings";

export function IssuanceMethodBadge({ method }: { method: IssuanceMethod }) {
  return method === "acme" ? (
    <Badge variant="outline">
      <KeyRound className="size-3" />
      ACME
    </Badge>
  ) : (
    <Badge variant="outline">
      <Landmark className="size-3" />
      自签
    </Badge>
  );
}

export function ValidationMethodBadge({ method }: { method: ValidationMethod | null }) {
  if (!method) return <span className="text-muted-foreground">—</span>;
  return <Badge variant="outline">{method === "http_01" ? "HTTP-01" : "DNS-01"}</Badge>;
}

export function WildcardBadge() {
  return <Badge variant="outline">通配符</Badge>;
}

/** CA 环境(§3.5:生产=中性 outline;测试=warning outline 弱提示非生产)。label 为后端展示串,可空。 */
export function EnvironmentBadge({ environment }: { environment: string | null }) {
  if (!environment) return <span className="text-muted-foreground">—</span>;
  const isTest = /测试|test|staging/i.test(environment);
  return <Badge variant={isTest ? "outline-warning" : "outline"}>{environment}</Badge>;
}

/** 创建方式(§3.5:新建 / 导入)—— 中性 outline。局部属性 wire 值 created/imported。 */
export function CreationMethodBadge({ method }: { method: string }) {
  return <Badge variant="outline">{method === "imported" ? "导入" : "新建"}</Badge>;
}

const TASK_TYPE_LABEL: Record<TaskType, string> = {
  issue: "签发",
  renew: "续签",
  revoke: "吊销",
};

const TASK_TRIGGER_LABEL: Record<TaskTrigger, string> = {
  manual: "手动",
  auto: "自动",
  cleanup: "清理",
};

/** 任务类型中文名(单一来源)—— 供筛选下拉复用。 */
export function taskTypeLabel(t: TaskType): string {
  return TASK_TYPE_LABEL[t] ?? t;
}

/** 触发方式中文名(单一来源)—— 供筛选下拉复用。 */
export function taskTriggerLabel(t: TaskTrigger): string {
  return TASK_TRIGGER_LABEL[t] ?? t;
}

/** 任务类型(§3.5 分类语义)—— 中性 outline,不占状态色。 */
export function TaskTypeBadge({ type }: { type: TaskType }) {
  return <Badge variant="outline">{taskTypeLabel(type)}</Badge>;
}

/** 触发方式(§3.5:手动 / 自动 / 清理)—— 中性 outline。 */
export function TaskTriggerBadge({ trigger }: { trigger: TaskTrigger }) {
  return <Badge variant="outline">{taskTriggerLabel(trigger)}</Badge>;
}
