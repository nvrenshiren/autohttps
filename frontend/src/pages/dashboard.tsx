import { Link, useNavigate } from "react-router";
import {
  ArrowRight,
  ArrowUpRight,
  CheckCircle2,
  Clock3,
  Globe2,
  ListChecks,
  Plus,
  ShieldAlert,
  ShieldCheck,
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
import { Button } from "@/components/ui/button";

function SummaryMetric({
  label,
  value,
  note,
  Icon,
  tone,
}: {
  label: string;
  value: number;
  note: string;
  Icon: LucideIcon;
  tone?: "warning" | "danger";
}) {
  const active = value > 0 && tone;
  const toneClass = active === "danger" ? "text-danger" : active === "warning" ? "text-warning-muted-foreground" : "text-foreground";
  return (
    <div className="min-w-0 px-4 py-2 first:pl-0 last:pr-0 sm:px-5">
      <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
        <Icon aria-hidden="true" className={cn("size-4", active && toneClass)} />
        <span>{label}</span>
      </div>
      <div className={cn("mt-2 font-display text-3xl font-semibold leading-none tracking-tight tabular-nums sm:text-4xl", toneClass)}>
        {value}
      </div>
      <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{note}</p>
    </div>
  );
}

function PendingRow({ item }: { item: PendingCertItem }) {
  const navigate = useNavigate();
  const hostname = item.domains[0] ?? "(无域名)";

  return (
    <li>
      <button
        type="button"
        className="group flex min-h-[68px] w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-inset focus-visible:ring-ring sm:px-5"
        onClick={() => navigate(`/certificates/${item.certificateId}`)}
      >
        <span aria-hidden="true" className="size-2 shrink-0 rounded-full bg-border-strong" />
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 items-center gap-2">
            <Mono className="max-w-full truncate font-medium text-foreground">{hostname}</Mono>
            {item.domains.length > 1 && <span className="shrink-0 text-xs text-muted-foreground">+{item.domains.length - 1}</span>}
          </span>
          <span className="mt-1 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
            <StatusBadge status={item.status} />
            {item.notAfter ? (
              <Tooltip>
                <TooltipTrigger asChild><span className="cursor-help">{daysLabel(item.daysUntilExpiry)}</span></TooltipTrigger>
                <TooltipContent><Mono>{absoluteUtc(item.notAfter)}</Mono></TooltipContent>
              </Tooltip>
            ) : <span>尚未签发</span>}
          </span>
        </span>
        <ArrowRight aria-hidden="true" className="size-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-foreground" />
      </button>
    </li>
  );
}

function OperationLink({
  to,
  label,
  detail,
  Icon,
}: {
  to: string;
  label: string;
  detail: string;
  Icon: LucideIcon;
}) {
  return (
    <Link to={to} className="group flex min-h-[58px] items-center gap-3 rounded-md px-2.5 py-2 transition-colors hover:bg-muted/70 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring">
      <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground transition-colors group-hover:border-primary/25 group-hover:text-primary">
        <Icon aria-hidden="true" className="size-[17px]" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-medium">{label}</span>
        <span className="mt-0.5 block truncate text-xs text-muted-foreground">{detail}</span>
      </span>
      <ArrowUpRight aria-hidden="true" className="size-4 shrink-0 text-muted-foreground transition-colors group-hover:text-primary" />
    </Link>
  );
}

export function DashboardPage() {
  const navigate = useNavigate();
  const { data, isLoading, isError, error, refetch } = useDashboard();

  return (
    <div className="mx-auto w-full max-w-[1440px] space-y-6 p-4 sm:p-6 lg:p-8">
      <PageHeader title="运行概览" description="查看证书健康状态，并直接进入需要处理的事项。" />

      <div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1.7fr)_minmax(270px,0.8fr)]">
        <div className="min-w-0 space-y-5">
          <section aria-label="证书运行指标" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-4 sm:px-6">
              <div className="flex items-center gap-2.5">
                <span className="inline-flex size-8 items-center justify-center rounded-md bg-accent text-primary"><ShieldCheck aria-hidden="true" className="size-4" /></span>
                <div>
                  <h2 className="text-sm font-semibold">证书保障状态</h2>
                  <p className="mt-0.5 text-xs text-muted-foreground">当前实例中的证书资产</p>
                </div>
              </div>
            </div>

            {isLoading ? (
              <div className="grid grid-cols-3 divide-x divide-border px-5 py-5 sm:px-6">
                {[0, 1, 2].map((i) => <Skeleton key={i} className="h-20 rounded-md" />)}
              </div>
            ) : isError ? (
              <div className="p-5"><ErrorState error={error} onRetry={() => void refetch()} /></div>
            ) : data ? (
              <dl className="grid grid-cols-3 divide-x divide-border px-5 py-5 sm:px-6">
                <SummaryMetric label="证书总数" value={data.metrics.totalCount} note="包含全部状态" Icon={ShieldCheck} />
                <SummaryMetric label="即将到期" value={data.metrics.expiringSoonCount} note="按续签策略判定" Icon={Clock3} tone="warning" />
                <SummaryMetric label="失败" value={data.metrics.failedCount} note="含签发、续签失败与过期" Icon={ShieldAlert} tone="danger" />
              </dl>
            ) : null}
          </section>

          {!isLoading && !isError && data && (
            <section aria-labelledby="pending-certificates-title" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
              <div className="flex min-h-[62px] items-center justify-between gap-3 border-b border-border px-4 py-3 sm:px-5">
                <div className="flex items-center gap-2.5">
                  <h2 id="pending-certificates-title" className="text-sm font-semibold">待处理</h2>
                  <span className={cn("inline-flex h-6 min-w-6 items-center justify-center rounded-full px-2 text-xs font-semibold tabular-nums", data.pendingCount > 0 ? "bg-danger-muted text-danger-muted-foreground" : "bg-muted text-muted-foreground")}>
                    {data.pendingCount}
                  </span>
                </div>
                <Link to="/certificates" className="inline-flex min-h-9 items-center gap-1 rounded-sm px-2 text-xs font-medium text-link transition-colors hover:text-link-hover focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring">
                  全部证书<ArrowRight aria-hidden="true" className="size-3.5" />
                </Link>
              </div>
              {data.pendingItems.length === 0 ? (
                <EmptyState Icon={CheckCircle2} iconClassName="text-success" title="当前没有待处理证书" description="到期关注和失败状态会在这里集中显示。" />
              ) : (
                <ul className="divide-y divide-border">
                  {data.pendingItems.map((item) => <PendingRow key={item.certificateId} item={item} />)}
                </ul>
              )}
            </section>
          )}
        </div>

        <aside aria-labelledby="operations-title" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
          <div className="border-b border-border px-5 py-4">
            <div className="flex items-center gap-2.5">
              <span className="inline-flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground"><Plus aria-hidden="true" className="size-4" /></span>
              <div>
                <h2 id="operations-title" className="text-sm font-semibold">常用操作</h2>
                <p className="mt-0.5 text-xs text-muted-foreground">从这里开始下一步工作</p>
              </div>
            </div>
          </div>
          <div className="p-3">
            <Button className="mb-2 min-h-11 w-full justify-between" onClick={() => navigate("/certificates/issue")}>
              <span className="inline-flex items-center gap-2"><Plus aria-hidden="true" />发起证书签发</span>
              <ArrowRight aria-hidden="true" className="size-4" />
            </Button>
            <OperationLink to="/domains" label="域名管理" detail="维护主机名与验证方式" Icon={Globe2} />
            <OperationLink to="/tasks" label="执行记录" detail="查看签发、续签与失败原因" Icon={ListChecks} />
          </div>
          <div className="border-t border-border bg-muted/30 px-5 py-3 text-xs leading-relaxed text-muted-foreground">
            证书状态由后台自动扫描；此页面只读汇总并提供处理入口。
          </div>
        </aside>
      </div>
    </div>
  );
}
