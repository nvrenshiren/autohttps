import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { ArrowUpRight, CalendarClock, Plus, Search, ShieldCheck } from "lucide-react";
import { useCertificates } from "@/lib/queries";
import { CERTIFICATE_STATUSES, ISSUANCE_METHODS } from "@/bindings";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { StatusBadge, statusLabel } from "@/components/status-badge";
import { IssuanceMethodBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";

const PAGE_SIZE = 20;
const ALL = "all";

export function CertificatesListPage() {
  const navigate = useNavigate();
  const [status, setStatus] = useState<string>(ALL);
  const [method, setMethod] = useState<string>(ALL);
  const [domain, setDomain] = useState("");
  const [page, setPage] = useState(1);
  const hasFilter = status !== ALL || method !== ALL || domain !== "";

  const { data, isLoading, isError, error, refetch } = useCertificates({
    page,
    pageSize: PAGE_SIZE,
    status: status === ALL ? undefined : status,
    issuanceMethod: method === ALL ? undefined : method,
    domain: domain || undefined,
  });

  const items = data?.items ?? [];
  const showEmpty = !isLoading && !isError && items.length === 0;
  const resetFilters = () => { setStatus(ALL); setMethod(ALL); setDomain(""); setPage(1); };

  return (
    <div className="mx-auto w-full max-w-[1440px] p-4 sm:p-6 lg:p-8">
      <PageHeader
        title="证书资产"
        description="按域名、状态和签发来源查找证书；生命周期操作在证书详情中完成。"
        actions={<Button onClick={() => navigate("/certificates/issue")}><Plus />发起签发</Button>}
      />

      <section aria-label="证书筛选与列表" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
        <div className="flex flex-wrap items-center gap-2.5 border-b border-border bg-muted/25 px-4 py-3 sm:px-5">
          <div className="relative min-w-[190px] flex-1 sm:max-w-[280px]">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input aria-label="按域名搜索证书" placeholder="搜索域名" className="pl-9" value={domain} onChange={(event) => { setDomain(event.target.value); setPage(1); }} />
          </div>
          <Select value={status} onValueChange={(value) => { setStatus(value); setPage(1); }}>
            <SelectTrigger aria-label="按证书状态筛选" className="min-w-[132px] flex-1 sm:flex-none"><SelectValue placeholder="状态" /></SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部状态</SelectItem>
              {CERTIFICATE_STATUSES.map((state) => <SelectItem key={state} value={state}>{statusLabel(state)}</SelectItem>)}
            </SelectContent>
          </Select>
          <Select value={method} onValueChange={(value) => { setMethod(value); setPage(1); }}>
            <SelectTrigger aria-label="按签发方式筛选" className="min-w-[132px] flex-1 sm:flex-none"><SelectValue placeholder="签发方式" /></SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部来源</SelectItem>
              {ISSUANCE_METHODS.map((item) => <SelectItem key={item} value={item}>{item === "acme" ? "公共 ACME" : "本地根 CA"}</SelectItem>)}
            </SelectContent>
          </Select>
          {hasFilter && <Button variant="ghost" size="sm" onClick={resetFilters}>清除筛选</Button>}
          <span className="ml-auto inline-flex min-h-9 items-center text-xs text-muted-foreground" aria-live="polite">{data ? `${data.total} 张证书` : ""}</span>
        </div>

        {isError ? (
          <div className="p-5"><ErrorState error={error} onRetry={() => void refetch()} /></div>
        ) : showEmpty ? (
          hasFilter ? (
            <EmptyState Icon={Search} title="没有匹配的证书" description="尝试调整域名、状态或签发来源筛选。" action={<Button variant="outline" size="sm" onClick={resetFilters}>清除筛选</Button>} />
          ) : (
            <EmptyState Icon={ShieldCheck} title="尚无证书" description="选择已登记域名，通过公共 ACME 或本地根 CA 发起签发。" action={<Button onClick={() => navigate("/certificates/issue")}><Plus />发起签发</Button>} />
          )
        ) : (
          <ul className="divide-y divide-border">
            {isLoading ? Array.from({ length: 6 }, (_, index) => (
              <li key={index} className="flex min-h-[92px] items-center gap-4 px-4 py-4 sm:px-5">
                <Skeleton className="size-10 shrink-0 rounded-md" />
                <div className="flex-1 space-y-2"><Skeleton className="h-4 w-56" /><Skeleton className="h-3 w-36" /></div>
                <Skeleton className="h-7 w-24" />
              </li>
            )) : items.map((certificate) => (
              <li key={certificate.id} className="group flex min-w-0 items-center gap-2 px-3 py-2 transition-colors hover:bg-muted/30 sm:gap-3 sm:px-4">
                <Link to={`/certificates/${certificate.id}`} aria-label={`查看证书详情：${certificate.domains.map((item) => item.hostname).join("、") || "无关联域名"}`} className="flex min-h-[78px] min-w-0 flex-1 items-center gap-3 rounded-md px-1.5 py-2 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring sm:gap-4 sm:px-2">
                  <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground group-hover:text-primary"><ShieldCheck aria-hidden="true" className="size-[18px]" /></span>
                  <span className="min-w-0 flex-1">
                    <span className="flex flex-wrap items-center gap-2">
                      {certificate.domains.length === 0 ? <span className="text-sm font-medium">未关联域名</span> : certificate.domains.map((item) => <Mono key={item.id} className="max-w-full truncate font-medium text-foreground group-hover:text-link">{item.hostname}</Mono>)}
                    </span>
                    <span className="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
                      <IssuanceMethodBadge method={certificate.issuanceMethod} />
                      <span>更新于 {relativeTime(certificate.updatedAt)}</span>
                    </span>
                  </span>
                  <ArrowUpRight aria-hidden="true" className="hidden size-4 shrink-0 text-muted-foreground transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5 sm:block" />
                </Link>

                <div className="flex min-w-[120px] shrink-0 flex-col items-start gap-1.5 sm:min-w-[148px] sm:items-end sm:gap-2">
                  <StatusBadge status={certificate.status} />
                  {certificate.notAfter ? (
                    <Tooltip><TooltipTrigger asChild><span className="inline-flex items-center gap-1 text-xs text-muted-foreground"><CalendarClock aria-hidden="true" className="size-3.5" />{daysLabel(certificate.daysUntilExpiry)}</span></TooltipTrigger><TooltipContent><Mono>{absoluteUtc(certificate.notAfter)}</Mono></TooltipContent></Tooltip>
                  ) : <span className="text-xs text-muted-foreground">尚未签发</span>}
                </div>
              </li>
            ))}
          </ul>
        )}

        {data && data.total > PAGE_SIZE && <Pagination page={page} pageSize={PAGE_SIZE} total={data.total} onPage={setPage} />}
      </section>
    </div>
  );
}
