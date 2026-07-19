/**
 * 任务列表(tasks/list PRD)—— 队列 + 历史统一一表(DEC1);类型 / 状态 / 关联证书 / 时间四类可组合筛选
 * (F2–F5)。行内进详情(F6)。重试仅 failed、取消仅 queued/running(H4 禁用 + Tooltip);二次确认后
 * 接真端点(取消驱动证书回退)。无批量(H12)。四态齐备(H3)。
 */
import { useState } from "react";
import { useNavigate, useSearchParams } from "react-router";
import { ListChecks, MoreHorizontal, RotateCw, Search, XCircle } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { qk, useTasks } from "@/lib/queries";
import type { TaskSummary } from "@/bindings";
import { TASK_TYPES } from "@/bindings";
import { api, ApiError } from "@/lib/api";
import { canCancelTask, canRetryTask } from "@/lib/task-rules";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState, TableSkeletonRows } from "@/components/shared/states";
import { Pagination } from "@/components/shared/pagination";
import { StatusBadge, statusLabel } from "@/components/status-badge";
import { TaskTypeBadge, TaskTriggerBadge, taskTypeLabel } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
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
import { absoluteUtc, relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

const PAGE_SIZE = 20;
const COLS = 6;
const ALL = "all";
const QUEUE = "queue"; // 聚合:排队 + 执行中(DEC1)

/** 日期(YYYY-MM-DD)→ RFC3339 UTC 区间端点。 */
function toIso(date: string, end: boolean): string | undefined {
  if (!date) return undefined;
  return `${date}T${end ? "23:59:59" : "00:00:00"}Z`;
}

type ConfirmAction = { task: TaskSummary; kind: "retry" | "cancel" };

export function TasksListPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const certificateId = searchParams.get("certificateId") ?? undefined;

  const [type, setType] = useState<string>(ALL);
  const [status, setStatus] = useState<string>(ALL);
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [page, setPage] = useState(1);
  const [confirm, setConfirm] = useState<ConfirmAction | null>(null);
  const [busy, setBusy] = useState(false);
  const qc = useQueryClient();

  const statusParam = status === ALL ? undefined : status === QUEUE ? "queued,running" : status;
  const hasFilter =
    type !== ALL || status !== ALL || dateFrom !== "" || dateTo !== "" || !!certificateId;

  const { data, isLoading, isError, error, refetch } = useTasks({
    page,
    pageSize: PAGE_SIZE,
    taskType: type === ALL ? undefined : type,
    status: statusParam,
    certificateId,
    dateFrom: toIso(dateFrom, false),
    dateTo: toIso(dateTo, true),
    sort: "queuedAt",
    order: "desc",
  });

  const items = data?.items ?? [];
  const showEmpty = !isLoading && !isError && items.length === 0;

  const resetFilters = () => {
    setType(ALL);
    setStatus(ALL);
    setDateFrom("");
    setDateTo("");
    setPage(1);
    if (certificateId) {
      searchParams.delete("certificateId");
      setSearchParams(searchParams, { replace: true });
    }
  };

  const runAction = async () => {
    if (!confirm) return;
    const { task, kind } = confirm;
    const label = kind === "retry" ? "重试" : "取消";
    setBusy(true);
    try {
      await api.post(`/tasks/${task.id}/${kind}`);
      toast.success(`${label}已发起`);
      void refetch();
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.dashboard });
      qc.invalidateQueries({ queryKey: qk.certificate(task.certificateId) });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : `${label}失败`);
    } finally {
      setBusy(false);
      setConfirm(null);
    }
  };

  return (
    <div className="p-4 sm:p-6">
      <PageHeader
        title="任务与历史"
        description="签发 / 续签 / 吊销任务的队列与历史统一视图;失败可重试、进行中可取消。"
      />

      <div className="overflow-hidden rounded-2xl border border-border bg-card shadow-card">
        {/* 工具栏(组合筛选:类型 / 状态 / 时间;关联证书经详情页跳入) */}
        <div className="flex flex-wrap items-center gap-2 border-b border-border p-3">
          <Select
            value={type}
            onValueChange={(v) => {
              setType(v);
              setPage(1);
            }}
          >
            <SelectTrigger size="sm" className="w-32">
              <SelectValue placeholder="类型" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部类型</SelectItem>
              {TASK_TYPES.map((t) => (
                <SelectItem key={t} value={t}>
                  {taskTypeLabel(t)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select
            value={status}
            onValueChange={(v) => {
              setStatus(v);
              setPage(1);
            }}
          >
            <SelectTrigger size="sm" className="w-40">
              <SelectValue placeholder="状态" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL}>全部状态</SelectItem>
              <SelectItem value={QUEUE}>队列(进行中)</SelectItem>
              <SelectItem value="queued">{statusLabel("queued")}</SelectItem>
              <SelectItem value="running">{statusLabel("running")}</SelectItem>
              <SelectItem value="succeeded">{statusLabel("succeeded")}</SelectItem>
              <SelectItem value="failed">{statusLabel("failed")}</SelectItem>
              <SelectItem value="cancelled">{statusLabel("cancelled")}</SelectItem>
            </SelectContent>
          </Select>
          <div className="flex items-center gap-1.5">
            <Label htmlFor="task-from" className="text-xs text-muted-foreground">
              入队
            </Label>
            <Input
              id="task-from"
              type="date"
              className="h-8 w-36"
              value={dateFrom}
              onChange={(e) => {
                setDateFrom(e.target.value);
                setPage(1);
              }}
            />
            <span className="text-xs text-muted-foreground">→</span>
            <Input
              type="date"
              className="h-8 w-36"
              value={dateTo}
              onChange={(e) => {
                setDateTo(e.target.value);
                setPage(1);
              }}
            />
          </div>
          {certificateId && (
            <Badge variant="outline" className="gap-1">
              关联证书 <Mono className="text-[11px]">{certificateId.slice(0, 8)}…</Mono>
            </Badge>
          )}
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
              title="没有匹配的任务"
              description="调整筛选条件试试。"
              action={
                <Button variant="outline" size="sm" onClick={resetFilters}>
                  清除筛选
                </Button>
              }
            />
          ) : (
            <EmptyState
              Icon={ListChecks}
              title="尚无任务"
              description="签发 / 续签 / 吊销证书时会在此登记任务;历史只增留痕。"
            />
          )
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>类型</TableHead>
                <TableHead>关联证书</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>触发</TableHead>
                <TableHead>入队</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableSkeletonRows rows={6} cols={COLS} />
              ) : (
                items.map((t) => {
                  const domains = t.certificateDomains ?? [];
                  const primary = domains[0];
                  const summary = t.status === "failed" ? t.failureReason : t.resultSummary;
                  return (
                    <TableRow
                      key={t.id}
                      className="cursor-pointer"
                      onClick={() => navigate(`/tasks/${t.id}`)}
                    >
                      <TableCell>
                        <div className="flex items-center gap-1.5">
                          <TaskTypeBadge type={t.taskType} />
                          {t.attemptNumber > 1 && (
                            <span className="text-xs text-muted-foreground">
                              #{t.attemptNumber}
                            </span>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        {t.certificateDeleted ? (
                          <Badge variant="neutral">证书已删除</Badge>
                        ) : primary ? (
                          <div className="min-w-0">
                            <div className="flex items-center gap-1.5">
                              <Mono className="max-w-[180px] truncate">{primary}</Mono>
                              {domains.length > 1 && (
                                <span className="text-xs text-muted-foreground">
                                  +{domains.length - 1}
                                </span>
                              )}
                            </div>
                            {summary && (
                              <div
                                className={
                                  "max-w-[220px] truncate text-xs " +
                                  (t.status === "failed"
                                    ? "text-danger-muted-foreground"
                                    : "text-muted-foreground")
                                }
                              >
                                {summary}
                              </div>
                            )}
                          </div>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell>
                        <StatusBadge status={t.status} />
                      </TableCell>
                      <TableCell>
                        <TaskTriggerBadge trigger={t.trigger} />
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span className="cursor-help">{relativeTime(t.queuedAt)}</span>
                          </TooltipTrigger>
                          <TooltipContent>
                            <Mono>{absoluteUtc(t.queuedAt)}</Mono>
                          </TooltipContent>
                        </Tooltip>
                      </TableCell>
                      <TableCell onClick={(e) => e.stopPropagation()}>
                        <RowActions
                          task={t}
                          onView={() => navigate(`/tasks/${t.id}`)}
                          onRetry={() => setConfirm({ task: t, kind: "retry" })}
                          onCancel={() => setConfirm({ task: t, kind: "cancel" })}
                        />
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

      <ConfirmActionDialog
        action={confirm}
        busy={busy}
        onOpenChange={(o) => !o && setConfirm(null)}
        onConfirm={() => void runAction()}
      />
    </div>
  );
}

function RowActions({
  task,
  onView,
  onRetry,
  onCancel,
}: {
  task: TaskSummary;
  onView: () => void;
  onRetry: () => void;
  onCancel: () => void;
}) {
  const retryable = canRetryTask(task.status);
  const cancellable = canCancelTask(task.status);
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="更多操作">
          <MoreHorizontal />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuItem onClick={onView}>查看详情</DropdownMenuItem>
        <DropdownMenuSeparator />
        <GatedItem
          enabled={retryable}
          reason="仅失败任务可重试"
          onClick={onRetry}
          Icon={RotateCw}
          label="重试"
        />
        <GatedItem
          enabled={cancellable}
          reason="仅排队 / 执行中任务可取消"
          onClick={onCancel}
          Icon={XCircle}
          label="取消"
        />
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function GatedItem({
  enabled,
  reason,
  onClick,
  Icon,
  label,
}: {
  enabled: boolean;
  reason: string;
  onClick: () => void;
  Icon: typeof RotateCw;
  label: string;
}) {
  if (enabled) {
    return (
      <DropdownMenuItem onClick={onClick}>
        <Icon />
        {label}
      </DropdownMenuItem>
    );
  }
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <DropdownMenuItem disabled onSelect={(e) => e.preventDefault()}>
          <Icon />
          {label}
        </DropdownMenuItem>
      </TooltipTrigger>
      <TooltipContent side="left">{reason}</TooltipContent>
    </Tooltip>
  );
}

export function ConfirmActionDialog({
  action,
  busy,
  onOpenChange,
  onConfirm,
}: {
  action: ConfirmAction | null;
  busy: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  const isRetry = action?.kind === "retry";
  return (
    <AlertDialog open={!!action} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{isRetry ? "重试该任务?" : "取消该任务?"}</AlertDialogTitle>
          <AlertDialogDescription>
            {isRetry
              ? "将派生一个新任务(同类型、同证书)入队,原失败任务保留于历史;并驱动关联证书从告警态回到进行中态。"
              : "将取消排队 / 执行中的任务。执行中取消为尽力而为——已提交至 CA 的在途操作可能仍生效,由证书模块下次扫描据实校正。"}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>返回</AlertDialogCancel>
          <AlertDialogAction
            className={
              isRetry ? undefined : "bg-destructive text-destructive-foreground hover:bg-destructive/90"
            }
            disabled={busy}
            onClick={(e) => {
              e.preventDefault();
              onConfirm();
            }}
          >
            {isRetry ? "重试" : "取消任务"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
