/**
 * 域名列表(domains/list PRD)—— 工具栏筛选 + 表格 + 证书态投影 + 行操作(删除硬拦截 DECD3)。
 * 四态齐备(H3);删除走 AlertDialog 二次确认(H5);被证书关联则禁用删除并 Tooltip 原因(H4)。
 */
import { useState } from "react";
import { useNavigate } from "react-router";
import { Globe, MoreHorizontal, Plus, Search, Trash2 } from "lucide-react";
import { useDeleteDomain, useDomains } from "@/lib/queries";
import type { DomainSummary } from "@/bindings";
import { ApiError } from "@/lib/api";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState, TableSkeletonRows } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { CertProjectionBadge } from "@/components/status-badge";
import { ValidationMethodBadge, WildcardBadge } from "@/components/shared/category-badges";
import { CreateDomainDialog } from "@/components/domains/create-domain-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Mono } from "@/components/shared/mono";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
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
const COLS = 6;

export function DomainsListPage() {
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<DomainSummary | null>(null);

  const { data, isLoading, isError, error, refetch, isPlaceholderData } = useDomains({
    page,
    pageSize: PAGE_SIZE,
    hostname: search || undefined,
    sort: "hostname",
    order: "asc",
  });
  const del = useDeleteDomain();

  const items = data?.items ?? [];
  const showEmpty = !isLoading && !isError && items.length === 0;

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
    <div className="p-4 sm:p-6">
      <PageHeader
        title="域名"
        description="维护域名与其验证方式;证书态为关联证书的投影。"
        actions={
          <Button onClick={() => setCreateOpen(true)}>
            <Plus />
            新增域名
          </Button>
        }
      />

      <div className="overflow-hidden rounded-xl border border-border bg-card">
        {/* 工具栏 */}
        <div className="flex flex-wrap items-center gap-2 border-b border-border p-3">
          <div className="relative w-full max-w-xs">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="搜索 hostname…"
              className="pl-8"
              value={search}
              onChange={(e) => {
                setSearch(e.target.value);
                setPage(1);
              }}
            />
          </div>
          {isPlaceholderData && <span className="text-xs text-muted-foreground">加载中…</span>}
        </div>

        {isError ? (
          <div className="p-4">
            <ErrorState error={error} onRetry={() => void refetch()} />
          </div>
        ) : showEmpty ? (
          search ? (
            <EmptyState
              Icon={Search}
              title="没有匹配的域名"
              description="调整搜索关键字试试。"
              action={
                <Button variant="outline" size="sm" onClick={() => setSearch("")}>
                  清除筛选
                </Button>
              }
            />
          ) : (
            <EmptyState
              Icon={Globe}
              title="尚无域名"
              description="新增第一个域名,之后即可为其签发证书。"
              action={
                <Button onClick={() => setCreateOpen(true)}>
                  <Plus />
                  新增域名
                </Button>
              }
            />
          )
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>hostname</TableHead>
                <TableHead>分组</TableHead>
                <TableHead>验证方式</TableHead>
                <TableHead>证书态</TableHead>
                <TableHead>更新</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableSkeletonRows rows={6} cols={COLS} />
              ) : (
                items.map((d) => {
                  const blockedDelete = d.certificateCount > 0;
                  return (
                    <TableRow
                      key={d.id}
                      className="cursor-pointer"
                      onClick={() => navigate(`/domains/${d.id}`)}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Mono className="max-w-[220px] truncate">{d.hostname}</Mono>
                          {d.isWildcard && <WildcardBadge />}
                        </div>
                      </TableCell>
                      <TableCell>
                        {d.groupName ? (
                          <span className="text-muted-foreground">{d.groupName}</span>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell>
                        <ValidationMethodBadge method={d.validationMethod} />
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <CertProjectionBadge status={d.worstCertificateStatus} />
                          {d.certificateCount > 0 && (
                            <span className="text-xs text-muted-foreground">
                              {d.certificateCount} 张
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {relativeTime(d.updatedAt)}
                      </TableCell>
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon" aria-label="更多操作">
                              <MoreHorizontal />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent>
                            <DropdownMenuItem onClick={() => navigate(`/domains/${d.id}`)}>
                              查看详情
                            </DropdownMenuItem>
                            {blockedDelete ? (
                              <Tooltip>
                                <TooltipTrigger asChild>
                                  <DropdownMenuItem
                                    variant="danger"
                                    disabled
                                    onSelect={(e) => e.preventDefault()}
                                  >
                                    <Trash2 />
                                    删除
                                  </DropdownMenuItem>
                                </TooltipTrigger>
                                <TooltipContent side="left">
                                  被 {d.certificateCount} 个证书关联,不可删除
                                </TooltipContent>
                              </Tooltip>
                            ) : (
                              <DropdownMenuItem variant="danger" onClick={() => setDeleteTarget(d)}>
                                <Trash2 />
                                删除
                              </DropdownMenuItem>
                            )}
                          </DropdownMenuContent>
                        </DropdownMenu>
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

      <CreateDomainDialog open={createOpen} onOpenChange={setCreateOpen} />

      <AlertDialog open={!!deleteTarget} onOpenChange={(o) => !o && setDeleteTarget(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>删除域名?</AlertDialogTitle>
            <AlertDialogDescription>
              将删除域名 <Mono>{deleteTarget?.hostname}</Mono> 及其元数据(分组 / 备注 / 验证配置)。
              此操作不可撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={del.isPending}>取消</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={(e) => {
                e.preventDefault();
                onDelete();
              }}
              disabled={del.isPending}
            >
              删除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
