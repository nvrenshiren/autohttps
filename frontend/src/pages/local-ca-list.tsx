/**
 * 根 CA 列表(local-ca/list PRD)—— 全部根 CA 一屏(多根并存 LC6)+ 新增入口 + 逐个导出 + 进详情。
 * 无筛选 / 无批量 / 无移除(H12:PRD F1–F4 未列筛选;LC5 不提供删除)。四态齐备(H3)。
 */
import { useState } from "react";
import { useNavigate } from "react-router";
import { Download, Landmark, Plus } from "lucide-react";
import { useAppInfo, useRootCas } from "@/lib/queries";
import type { RootCaSummary } from "@/bindings";
import { ApiError } from "@/lib/api";
import { downloadFile } from "@/lib/download";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState, TableSkeletonRows } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { StatusBadge } from "@/components/status-badge";
import { CreationMethodBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { absoluteUtc, daysLabel } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

const PAGE_SIZE = 100;
const COLS = 6;

export function RootCaListPage() {
  const navigate = useNavigate();
  const isDesktop = useAppInfo().data?.runMode === "desktop";
  const [page, setPage] = useState(1);
  const [exportingId, setExportingId] = useState<string | null>(null);

  const { data, isLoading, isError, error, refetch } = useRootCas({
    page,
    pageSize: PAGE_SIZE,
    sort: "notAfter",
    order: "asc",
  });

  const items = data?.items ?? [];
  const showEmpty = !isLoading && !isError && items.length === 0;

  const onExport = async (ca: RootCaSummary) => {
    setExportingId(ca.id);
    try {
      const saved = await downloadFile(`/root-cas/${ca.id}/export`, `root-ca-${ca.name || ca.id}.pem`, {
        desktop: isDesktop,
      });
      if (saved) toast.success("已导出根 CA 证书");
    } catch (e) {
      toast.error(e instanceof ApiError || e instanceof Error ? e.message : "导出失败");
    } finally {
      setExportingId(null);
    }
  };

  return (
    <div className="p-4 sm:p-6">
      <PageHeader
        title="根 CA"
        description="管理自签根 CA:创建 / 导入、导出公开证书供客户端信任。支持多根并存。"
        actions={
          <Button onClick={() => navigate("/local-ca/new")}>
            <Plus />
            新增根 CA
          </Button>
        }
      />

      <div className="overflow-hidden rounded-2xl border border-border bg-card shadow-card">
        {isError ? (
          <div className="p-4">
            <ErrorState error={error} onRetry={() => void refetch()} />
          </div>
        ) : showEmpty ? (
          <EmptyState
            Icon={Landmark}
            title="尚无根 CA"
            description="创建或导入第一个根 CA,之后即可用它签发内网证书。"
            action={
              <Button onClick={() => navigate("/local-ca/new")}>
                <Plus />
                新增根 CA
              </Button>
            }
          />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>名称</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>创建方式</TableHead>
                <TableHead>有效期</TableHead>
                <TableHead>指纹</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableSkeletonRows rows={4} cols={COLS} />
              ) : (
                items.map((ca) => (
                  <TableRow
                    key={ca.id}
                    className="cursor-pointer"
                    onClick={() => navigate(`/local-ca/${ca.id}`)}
                  >
                    <TableCell>
                      <span className="block max-w-[220px] truncate font-medium">{ca.name}</span>
                    </TableCell>
                    <TableCell>
                      <StatusBadge status={ca.status} />
                    </TableCell>
                    <TableCell>
                      <CreationMethodBadge method={ca.creationMethod} />
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span className="cursor-help">{daysLabel(ca.daysUntilExpiry)}</span>
                        </TooltipTrigger>
                        <TooltipContent>
                          <Mono>{absoluteUtc(ca.notAfter)}</Mono>
                        </TooltipContent>
                      </Tooltip>
                    </TableCell>
                    <TableCell>
                      {ca.fingerprint ? (
                        <Mono className="block max-w-[180px] truncate">{ca.fingerprint}</Mono>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell onClick={(e) => e.stopPropagation()}>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label={`导出 ${ca.name} 的根 CA 证书`}
                            disabled={exportingId === ca.id}
                            onClick={() => void onExport(ca)}
                          >
                            <Download />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent side="left">导出根 CA 证书(公开)</TooltipContent>
                      </Tooltip>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        )}

        {data && data.total > PAGE_SIZE && (
          <Pagination page={page} pageSize={PAGE_SIZE} total={data.total} onPage={setPage} />
        )}
      </div>

      {items.length > 0 && (
        <p className="mt-3 text-xs text-muted-foreground">
          共 {data?.total ?? items.length} 个根 CA · 已过期的根 CA 不可再签发新内网证书,需创建 / 导入新根接替。
        </p>
      )}
    </div>
  );
}
