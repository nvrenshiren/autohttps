/**
 * 总览(设计系统 v2)—— Hero 指标带(品牌渐变光晕 + 大数字)+ 待处理清单 + 常用操作。
 * 强调色仅在计数>0 出现(DS8);待处理告警级优先、已过期居首(§3.4,服务端已排序)。
 */
import { useNavigate } from "react-router";
import {
  ChevronRight,
  CircleCheckBig,
  Clock,
  Globe,
  ListChecks,
  Plus,
  ShieldCheck,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";
import { useDashboard } from "@/lib/queries";
import type { PendingCertItem } from "@/bindings";
import { Skeleton } from "@/components/ui/skeleton";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { Mono } from "@/components/shared/mono";
import { absoluteUtc, daysLabel } from "@/lib/time";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { PageHeader } from "@/components/shared/page-header";

function MetricCard({
  label,
  value,
  hint,
  Icon,
  emphasis,
  primary,
}: {
  label: string;
  value: number;
  hint: string;
  Icon: LucideIcon;
  emphasis?: "warning" | "danger";
  primary?: boolean;
}) {
  // 强调色仅计数>0 时出现(DS8);健康=中性
  const active = emphasis && value > 0;
  const color = active
    ? emphasis === "danger"
      ? "text-danger"
      : "text-warning"
    : undefined;
  const iconAccent = active && emphasis === "danger"
    ? "border-danger/20 bg-danger-muted text-danger"
    : active && emphasis === "warning"
      ? "border-warning/25 bg-warning-muted text-warning-muted-foreground"
      : primary
        ? "border-primary/15 bg-accent text-primary"
        : "border-border bg-muted text-muted-foreground";
  return (
    <div className="relative overflow-hidden rounded-xl border border-border bg-card p-4 shadow-card sm:p-5">
      <span
        aria-hidden="true"
        className={cn(
          "absolute inset-y-4 left-0 w-0.5 rounded-r-full",
          active && emphasis === "danger" ? "bg-danger" : active && emphasis === "warning" ? "bg-warning" : primary ? "bg-primary" : "bg-border",
        )}
      />
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-muted-foreground">{label}</span>
        <span
          className={cn(
            "inline-flex size-10 items-center justify-center rounded-md border transition-colors",
            iconAccent,
          )}
        >
          <Icon aria-hidden="true" className="size-[18px]" />
        </span>
      </div>
      <div
        className={cn("mt-3 font-display text-3xl font-semibold tabular-nums tracking-tight sm:text-4xl", color ?? (primary ? "text-primary" : "text-foreground"))}
      >
        {value}
      </div>
      <div className="relative mt-1.5 text-xs text-muted-foreground">{hint}</div>
    </div>
  );
}

function PendingRow({ item }: { item: PendingCertItem }) {
  const navigate = useNavigate();
  const failed =
    item.status === "expired" ||
    item.status === "issue_failed" ||
    item.status === "renewal_failed";
  const hostname = item.domains[0] ?? "(无域名)";
  return (
    <li>
      <button
        type="button"
        className="group relative flex w-full items-center gap-3 py-3 pl-5 pr-4 text-left transition-colors hover:bg-accent/50 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-inset focus-visible:ring-ring"
        onClick={() => navigate(`/certificates/${item.certificateId}`)}
      >
        <span className={cn("absolute bottom-2 left-0 top-2 w-0.5 rounded-full", failed ? "bg-danger" : "bg-warning")} />
        <div className="min-w-0 flex-1">
          <div className="truncate font-medium">
            {hostname}
            {item.domains.length > 1 && (
              <span className="ml-1 text-xs text-muted-foreground">+{item.domains.length - 1}</span>
            )}
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <StatusBadge status={item.status} />
            {item.notAfter ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="cursor-help">{daysLabel(item.daysUntilExpiry)}</span>
                </TooltipTrigger>
                <TooltipContent>
                  <Mono>{absoluteUtc(item.notAfter)}</Mono>
                </TooltipContent>
              </Tooltip>
            ) : (
              <span>尚未签发</span>
            )}
          </div>
        </div>
        <ChevronRight className="size-4 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
      </button>
    </li>
  );
}

function QuickAction({
  label,
  hint,
  Icon,
  onClick,
  primary,
}: {
  label: string;
  hint: string;
  Icon: LucideIcon;
  onClick: () => void;
  primary?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className="group flex items-center gap-3 rounded-xl border border-border bg-card p-4 text-left shadow-xs transition-[border-color,background-color,box-shadow,transform] duration-200 hover:-translate-y-0.5 hover:border-border-strong hover:bg-accent/30 hover:shadow-card focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring motion-reduce:transform-none"
    >
      <span
        className={cn(
          "inline-flex size-9 shrink-0 items-center justify-center rounded-xl transition-colors",
          primary
            ? "bg-primary text-primary-foreground"
            : "bg-secondary text-secondary-foreground",
        )}
      >
        <Icon aria-hidden="true" className="size-[18px]" />
      </span>
      <span className="min-w-0">
        <span className="block text-sm font-medium">{label}</span>
        <span className="block truncate text-xs text-muted-foreground">{hint}</span>
      </span>
    </button>
  );
}

export function DashboardPage() {
  const navigate = useNavigate();
  const { data, isLoading, isError, error, refetch } = useDashboard();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-7 p-4 sm:p-6 lg:p-8">
      <PageHeader
        title="证书概览"
        description="快速查看证书健康状态，并处理临期或签发异常。"
      />
      {/* Hero 指标带 */}
      {isLoading ? (
        <div className="grid gap-3 sm:grid-cols-3 lg:gap-4">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-[124px] rounded-xl" />
          ))}
        </div>
      ) : isError ? (
        <ErrorState error={error} onRetry={() => void refetch()} />
      ) : data ? (
        <section aria-label="证书指标" className="relative">
          <div className="grid gap-3 sm:grid-cols-3 lg:gap-4">
            <MetricCard
              label="证书总数"
              value={data.metrics.totalCount}
              hint="含各状态(含已吊销)"
              Icon={ShieldCheck}
              primary
            />
            <MetricCard
              label="即将到期"
              value={data.metrics.expiringSoonCount}
              hint="临近到期 · 需及时续签"
              Icon={Clock}
              emphasis="warning"
            />
            <MetricCard
              label="失败"
              value={data.metrics.failedCount}
              hint="已过期 / 签发失败 / 续签失败"
              Icon={TriangleAlert}
              emphasis="danger"
            />
          </div>
        </section>
      ) : null}

      {/* 待处理清单 */}
      {!isLoading && !isError && data && (
        <section aria-labelledby="pending-certificates-title" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
          <div className="flex items-center justify-between gap-3 border-b border-border bg-muted/20 px-5 py-4 sm:px-6">
            <div className="flex items-center gap-2">
              <h2 id="pending-certificates-title" className="font-display text-base font-semibold tracking-tight">
                待处理证书
              </h2>
              {data.pendingCount > 0 && (
                <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-md bg-danger-muted px-1.5 text-xs font-semibold text-danger-muted-foreground">
                  {data.pendingCount}
                </span>
              )}
            </div>
            {data.pendingCount > 0 && (
              <span className="hidden text-xs text-muted-foreground sm:inline">
                告警级优先 · 已过期居首
              </span>
            )}
          </div>
          {data.pendingItems.length === 0 ? (
            <EmptyState
              Icon={CircleCheckBig}
              iconClassName="text-success"
              title="全部证书状态良好"
              description="没有需要处理的证书,红点已清零。"
            />
          ) : (
            <ul className="divide-y divide-border">
              {data.pendingItems.map((item) => (
                <PendingRow key={item.certificateId} item={item} />
              ))}
            </ul>
          )}
        </section>
      )}

      {/* 常用操作 */}
      <section>
        <h2 className="mb-3 flex items-center gap-2 font-display text-base font-semibold tracking-tight">
          <span aria-hidden="true" className="h-4 w-0.5 rounded-full bg-primary" />
          快捷入口
        </h2>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <QuickAction
            label="签发证书"
            hint="创建新证书"
            Icon={Plus}
            primary
            onClick={() => navigate("/certificates/issue")}
          />
          <QuickAction
            label="全部证书"
            hint="证书列表与状态"
            Icon={ShieldCheck}
            onClick={() => navigate("/certificates")}
          />
          <QuickAction
            label="域名管理"
            hint="维护域名列表"
            Icon={Globe}
            onClick={() => navigate("/domains")}
          />
          <QuickAction
            label="任务与历史"
            hint="签发 / 续签 / 吊销记录"
            Icon={ListChecks}
            onClick={() => navigate("/tasks")}
          />
        </div>
      </section>
    </div>
  );
}
