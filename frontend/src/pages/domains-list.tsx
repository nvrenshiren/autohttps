import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { ArrowUpRight, Globe2, MoreHorizontal, Plus, Search, ShieldCheck, Trash2 } from "lucide-react";
import { useDeleteDomain, useDomains } from "@/lib/queries";
import type { DomainSummary } from "@/bindings";
import { CERTIFICATE_STATUSES } from "@/bindings";
import { ApiError } from "@/lib/api";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { CertProjectionBadge, statusLabel } from "@/components/status-badge";
import { ValidationMethodBadge, WildcardBadge } from "@/components/shared/category-badges";
import { CreateDomainDialog } from "@/components/domains/create-domain-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Mono } from "@/components/shared/mono";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

const PAGE_SIZE = 20;
const ALL = "all";

export function DomainsListPage() {
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [group, setGroup] = useState("");
  const [certificateState, setCertificateState] = useState(ALL);
  const [page, setPage] = useState(1);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<DomainSummary | null>(null);

  const certificateStateQuery = certificateState === "failures"
    ? "expired,issue_failed,renewal_failed"
    : certificateState === ALL ? undefined : certificateState;
  const { data, isLoading, isError, error, refetch, isPlaceholderData } = useDomains({
    page,
    pageSize: PAGE_SIZE,
    hostname: search || undefined,
    group: group || undefined,
    certificateState: certificateStateQuery,
    sort: "hostname",
    order: "asc",
  });
  const del = useDeleteDomain();
  const items = data?.items ?? [];
  const hasFilter = search !== "" || group !== "" || certificateState !== ALL;
  const showEmpty = !isLoading && !isError && items.length === 0;

  const resetFilters = () => {
    setSearch("");
    setGroup("");
    setCertificateState(ALL);
    setPage(1);
  };

  const onDelete = () => {
    if (!deleteTarget) return;
    del.mutate(deleteTarget.id, {
      onSuccess: () => {
        toast.success(`已删除域名 ${deleteTarget.hostname}`);
        setDeleteTarget(null);
      },
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "删除失败"),
    });
  };

  return (
    <div className="mx-auto w-full max-w-[1440px] p-4 sm:p-6 lg:p-8">
      <PageHeader
        title="域名资产"
        description="维护主机名、分组与验证偏好，并从域名侧查看证书覆盖情况。"
        actions={<Button onClick={() => setCreateOpen(true)}><Plus />新增域名</Button>}
      />

      <section aria-label="域名筛选与列表" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
        <div className="flex flex-wrap items-center gap-2.5 border-b border-border bg-muted/25 px-4 py-3 sm:px-5">
          <div className="relative min-w-[190px] flex-1 sm:max-w-[280px]">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input aria-label="按主机名筛选" placeholder="搜索主机名" className="pl-9" value={search} onChange={(e) => { setSearch(e.target.value); setPage(1); }} />
          </div>
          <div className="relative min-w-[150px] flex-1 sm:flex-none">
            <Input aria-label="按分组精确筛选" placeholder="分组名称" value={group} onChange={(e) => { setGroup(e.target.value); setPage(1); }} />
          </div>
          <Select value={certificateState} onValueChange={(value) => { setCertificateState(value); setPage(1); }}>
            <SelectTrigger aria-label="按证书状态筛选" className="min-w-[148px] flex-1 sm:flex-none"><SelectValue placeholder="证书状态" /></SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部证书状态</SelectItem>
              <SelectItem value="none">无证书</SelectItem>
              <SelectItem value="failures">失败或过期</SelectItem>
              {CERTIFICATE_STATUSES.filter((state) => !["expired", "issue_failed", "renewal_failed"].includes(state)).map((state) => (
                <SelectItem key={state} value={state}>{statusLabel(state)}</SelectItem>
              ))}
            </SelectContent>
          </Select>
          {hasFilter && <Button variant="ghost" size="sm" onClick={resetFilters}>清除筛选</Button>}
          <span className="ml-auto inline-flex min-h-9 items-center text-xs text-muted-foreground" aria-live="polite">
            {isPlaceholderData ? "正在更新…" : data ? `${data.total} 个域名` : ""}
          </span>
        </div>

        {isError ? (
          <div className="p-5"><ErrorState error={error} onRetry={() => void refetch()} /></div>
        ) : showEmpty ? (
          hasFilter ? (
            <EmptyState Icon={Search} title="没有匹配的域名" description="调整主机名、分组或证书状态筛选条件。" action={<Button variant="outline" size="sm" onClick={resetFilters}>清除筛选</Button>} />
          ) : (
            <EmptyState Icon={Globe2} title="尚无域名" description="先添加主机名，之后即可为它们签发证书。" action={<Button onClick={() => setCreateOpen(true)}><Plus />新增域名</Button>} />
          )
        ) : (
          <ul className="divide-y divide-border">
            {isLoading ? Array.from({ length: 6 }, (_, i) => (
              <li key={i} className="flex min-h-[84px] items-center gap-4 px-4 py-4 sm:px-5">
                <Skeleton className="size-10 shrink-0 rounded-md" />
                <div className="flex-1 space-y-2"><Skeleton className="h-4 w-52" /><Skeleton className="h-3 w-36" /></div>
                <Skeleton className="h-7 w-24" />
              </li>
            )) : items.map((domain) => {
              const blockedDelete = domain.certificateCount > 0;
              return (
                <li key={domain.id} className="group flex min-w-0 items-center gap-2 px-3 py-2 transition-colors hover:bg-muted/30 sm:gap-3 sm:px-4">
                  <Link to={`/domains/${domain.id}`} aria-label={`查看域名详情：${domain.hostname}`} className="flex min-h-[72px] min-w-0 flex-1 items-center gap-3 rounded-md px-1.5 py-2 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring sm:gap-4 sm:px-2">
                    <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-md border border-border bg-background text-muted-foreground group-hover:text-primary"><Globe2 aria-hidden="true" className="size-[18px]" /></span>
                    <span className="min-w-0 flex-1">
                      <span className="flex min-w-0 flex-wrap items-center gap-2">
                        <Mono className="max-w-full truncate font-medium text-foreground group-hover:text-link">{domain.hostname}</Mono>
                        {domain.isWildcard && <WildcardBadge />}
                      </span>
                      <span className="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
                        <span>{domain.groupName || "未分组"}</span>
                        <span aria-hidden="true" className="text-border-strong">·</span>
                        <span className="max-w-[28ch] truncate">{domain.remark || "无备注"}</span>
                      </span>
                      <span className="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
                        <ValidationMethodBadge method={domain.validationMethod} />
                        <span aria-hidden="true" className="text-border-strong">·</span>
                        <span>更新于 {relativeTime(domain.updatedAt)}</span>
                      </span>
                    </span>
                    <ArrowUpRight aria-hidden="true" className="hidden size-4 shrink-0 text-muted-foreground transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5 sm:block" />
                  </Link>

                  <div className="flex min-w-[112px] shrink-0 flex-col items-start gap-1.5 sm:min-w-[154px] sm:flex-row sm:items-center sm:justify-end sm:gap-2.5">
                    <CertProjectionBadge status={domain.worstCertificateStatus} />
                    <div className="flex items-center gap-1.5 text-xs text-muted-foreground" title={`${domain.certificateCount} 张关联证书`}>
                      <ShieldCheck aria-hidden="true" className="size-3.5" />{domain.certificateCount} 张证书
                    </div>
                  </div>

                  <DropdownMenu>
                    <DropdownMenuTrigger asChild><Button variant="ghost" size="icon" aria-label={`更多操作：${domain.hostname}`}><MoreHorizontal /></Button></DropdownMenuTrigger>
                    <DropdownMenuContent>
                      <DropdownMenuItem onClick={() => navigate(`/domains/${domain.id}`)}>查看详情</DropdownMenuItem>
                      {blockedDelete ? (
                        <Tooltip>
                          <TooltipTrigger asChild><DropdownMenuItem variant="danger" disabled onSelect={(e) => e.preventDefault()}><Trash2 />删除</DropdownMenuItem></TooltipTrigger>
                          <TooltipContent side="left">被 {domain.certificateCount} 张证书关联，暂不可删除</TooltipContent>
                        </Tooltip>
                      ) : <DropdownMenuItem variant="danger" onClick={() => setDeleteTarget(domain)}><Trash2 />删除</DropdownMenuItem>}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </li>
              );
            })}
          </ul>
        )}

        {data && data.total > PAGE_SIZE && <Pagination page={page} pageSize={PAGE_SIZE} total={data.total} onPage={setPage} />}
      </section>

      <CreateDomainDialog open={createOpen} onOpenChange={setCreateOpen} />
      <AlertDialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>删除域名？</AlertDialogTitle>
            <AlertDialogDescription>将删除域名 <Mono>{deleteTarget?.hostname}</Mono> 及其分组、备注和验证方式关联。此操作不可撤销。</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={del.isPending}>取消</AlertDialogCancel>
            <AlertDialogAction className="bg-destructive-solid text-destructive-foreground hover:bg-destructive-hover active:bg-destructive-active" onClick={(event) => { event.preventDefault(); onDelete(); }} disabled={del.isPending}>删除</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
