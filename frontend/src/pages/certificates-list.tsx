/**
 * 证书列表(certificates/list PRD)—— 状态 / 签发方式 / 域名筛选(可组合)+ 表格 + 四态。
 * 生命周期操作在详情页(H4);列表行点击进详情。禁批量(H12)。
 */
import { useState } from "react";
import { useNavigate } from "react-router";
import { Search, ShieldCheck } from "lucide-react";
import { useCertificates } from "@/lib/queries";
import { CERTIFICATE_STATUSES, ISSUANCE_METHODS } from "@/bindings";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState, TableSkeletonRows } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { StatusBadge, statusLabel } from "@/components/status-badge";
import { IssuanceMethodBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";

const PAGE_SIZE = 20;
const COLS = 5;
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

  const resetFilters = () => {
    setStatus(ALL);
    setMethod(ALL);
    setDomain("");
    setPage(1);
  };

  return (
    <div className="p-4 sm:p-6">
      <PageHeader
        title="证书"
        description="签发、续签、吊销证书的生命周期管理。"
      />

      <div className="overflow-hidden rounded-xl border border-border bg-card">
        {/* 工具栏(组合筛选) */}
        <div className="flex flex-wrap items-center gap-2 border-b border-border p-3">
          <div className="relative w-full max-w-xs">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="按域名搜索…"
              className="pl-8"
              value={domain}
              onChange={(e) => {
                setDomain(e.target.value);
                setPage(1);
              }}
            />
          </div>
          <Select
            value={status}
            onValueChange={(v) => {
              setStatus(v);
              setPage(1);
            }}
          >
            <SelectTrigger size="sm" className="w-36">
              <SelectValue placeholder="状态" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部状态</SelectItem>
              {CERTIFICATE_STATUSES.map((s) => (
                <SelectItem key={s} value={s}>
                  {statusLabel(s)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={method}
            onValueChange={(v) => {
              setMethod(v);
              setPage(1);
            }}
          >
            <SelectTrigger size="sm" className="w-32">
              <SelectValue placeholder="签发方式" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部方式</SelectItem>
              {ISSUANCE_METHODS.map((m) => (
                <SelectItem key={m} value={m}>
                  {m === "acme" ? "ACME" : "自签"}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {hasFilter && (
            <Button variant="ghost" size="sm" onClick={resetFilters}>
              清除筛选
            </Button>
          )}
        </div>

        {isError ? (
          <div className="p-4">
            <ErrorState error={error} onRetry={() => void refetch()} />
          </div>
        ) : showEmpty ? (
          hasFilter ? (
            <EmptyState
              Icon={Search}
              title="没有匹配的证书"
              description="调整筛选条件试试。"
              action={
                <Button variant="outline" size="sm" onClick={resetFilters}>
                  清除筛选
                </Button>
              }
            />
          ) : (
            <EmptyState
              Icon={ShieldCheck}
              title="尚无证书"
              description="通过「发起签发」为域名签发证书(需先配置 ACME 账户或根 CA;签发向导建设中)。"
            />
          )
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>域名</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>签发方式</TableHead>
                <TableHead>到期</TableHead>
                <TableHead>更新</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableSkeletonRows rows={6} cols={COLS} />
              ) : (
                items.map((c) => {
                  const primary = c.domains[0]?.hostname ?? "(无域名)";
                  return (
                    <TableRow
                      key={c.id}
                      className="cursor-pointer"
                      onClick={() => navigate(`/certificates/${c.id}`)}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Mono className="max-w-[220px] truncate">{primary}</Mono>
                          {c.domains.length > 1 && (
                            <span className="text-xs text-muted-foreground">
                              +{c.domains.length - 1}
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={c.status} />
                      </TableCell>
                      <TableCell>
                        <IssuanceMethodBadge method={c.issuanceMethod} />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {c.notAfter ? (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <span className="cursor-help">{daysLabel(c.daysUntilExpiry)}</span>
                            </TooltipTrigger>
                            <TooltipContent>
                              <Mono>{absoluteUtc(c.notAfter)}</Mono>
                            </TooltipContent>
                          </Tooltip>
                        ) : (
                          <span>未签发</span>
                        )}
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {relativeTime(c.updatedAt)}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        )}

        {data && data.total > PAGE_SIZE && (
          <Pagination page={page} pageSize={PAGE_SIZE} total={data.total} onPage={setPage} />
        )}
      </div>
    </div>
  );
}
