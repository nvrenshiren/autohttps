/**
 * StatusBadge —— 状态色语义**单一入口**(设计 §3 / DS2)。5 台状态机全部态 → 5 语义级 + lucide 图标。
 *
 * 这是 protocolLint L1 允许的唯一"状态字面量映射"落点(§3.2 的投影):其余组件禁手搓状态色/映射,
 * 一律经此渲染。wire 值跨状态机的碰撞(failed/cancelled/expired)映射到同一 §3.2 语义,故用扁平表。
 */
import {
  Ban,
  CircleCheckBig,
  CircleDashed,
  CircleSlash,
  CircleX,
  Clock,
  Hourglass,
  LoaderCircle,
  ShieldAlert,
  ShieldCheck,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";
import { Badge, type BadgeVariant } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { CertificateStatus } from "@/bindings";

interface StatusMeta {
  label: string;
  variant: BadgeVariant;
  Icon: LucideIcon;
  spin?: boolean;
}

// 逐态立法(§3.2)。info 内:排队/待开始用静态 Clock;执行中用旋转 LoaderCircle(DS3)。
const STATUS_META: Record<string, StatusMeta> = {
  // 证书 10 态
  pending_issue: { label: "待签发", variant: "info", Icon: Clock },
  issuing: { label: "签发中", variant: "info", Icon: LoaderCircle, spin: true },
  issue_failed: { label: "签发失败", variant: "danger", Icon: TriangleAlert },
  valid: { label: "有效", variant: "success", Icon: CircleCheckBig },
  expiring_soon: { label: "即将到期", variant: "warning", Icon: Clock },
  renewing: { label: "续签中", variant: "info", Icon: LoaderCircle, spin: true },
  renewal_failed: { label: "续签失败", variant: "danger", Icon: TriangleAlert },
  expired: { label: "已过期", variant: "danger", Icon: ShieldAlert },
  revoking: { label: "吊销中", variant: "info", Icon: LoaderCircle, spin: true },
  revoked: { label: "已吊销", variant: "neutral", Icon: Ban },
  // 任务 5 态
  queued: { label: "排队", variant: "info", Icon: Clock },
  running: { label: "执行中", variant: "info", Icon: LoaderCircle, spin: true },
  succeeded: { label: "成功", variant: "success", Icon: CircleCheckBig },
  failed: { label: "失败", variant: "danger", Icon: CircleX },
  cancelled: { label: "已取消", variant: "neutral", Icon: CircleSlash },
  // 挑战 6 态(pending/validating/passed 见上/下;awaiting_manual 归 warning,DS4)
  pending: { label: "待验证", variant: "info", Icon: Clock },
  awaiting_manual: { label: "等待手动配置", variant: "warning", Icon: Hourglass },
  validating: { label: "验证中", variant: "info", Icon: LoaderCircle, spin: true },
  passed: { label: "验证通过", variant: "success", Icon: CircleCheckBig },
  // 账户 4 态
  unconfigured: { label: "未配置", variant: "neutral", Icon: CircleDashed },
  registering: { label: "注册中", variant: "info", Icon: LoaderCircle, spin: true },
  registered: { label: "已注册", variant: "success", Icon: CircleCheckBig },
  registration_failed: { label: "注册失败", variant: "danger", Icon: CircleX },
  // 根 CA 2 态
  active: { label: "有效", variant: "success", Icon: ShieldCheck },
  // expired 复用证书 expired 语义(danger/ShieldAlert),已在上方定义
};

/** 状态中文名(§3.2 单一来源)—— 供筛选下拉等复用,避免散落硬写。 */
export function statusLabel(status: string): string {
  return STATUS_META[status]?.label ?? status;
}

export function StatusBadge({ status, className }: { status: string; className?: string }) {
  const meta = STATUS_META[status];
  if (!meta) {
    return (
      <Badge variant="neutral" className={className}>
        {status}
      </Badge>
    );
  }
  const { label, variant, Icon, spin } = meta;
  return (
    <Badge variant={variant} className={className}>
      <Icon className={cn("size-3 shrink-0", spin && "animate-spin")} />
      {label}
    </Badge>
  );
}

const FAILED_PROJECTION: CertificateStatus[] = ["expired", "issue_failed", "renewal_failed"];

/**
 * 域名列表「证书态投影」(§3.3):失败 > 即将到期 > 有效 > 无证书。
 * 进行中/待签/已吊销等其余态回落到实际 StatusBadge。
 */
export function CertProjectionBadge({ status }: { status: CertificateStatus | null }) {
  if (status === null) {
    return <Badge variant="outline">无证书</Badge>;
  }
  if (FAILED_PROJECTION.includes(status)) {
    return (
      <Badge variant="danger">
        <TriangleAlert className="size-3 shrink-0" />
        失败
      </Badge>
    );
  }
  if (status === "expiring_soon") {
    return (
      <Badge variant="warning">
        <Clock className="size-3 shrink-0" />
        即将到期
      </Badge>
    );
  }
  if (status === "valid") {
    return (
      <Badge variant="success">
        <CircleCheckBig className="size-3 shrink-0" />
        有效
      </Badge>
    );
  }
  return <StatusBadge status={status} />;
}
