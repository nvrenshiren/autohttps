/**
 * 任务详情(tasks/detail PRD)—— 完整信息 + 执行日志(mono、脱敏,H8)+ 重试链(前序 / 后继逐条可跳)。
 * 重试仅 failed、取消仅 queued/running(H4 禁用 + Tooltip);二次确认后接真端点(取消驱动证书回退)。
 * 关联证书可跳其详情;证书已删除则标注(DEC3)。
 */
import { useState } from "react";
import { useNavigate, useParams } from "react-router";
import { ListChecks, Loader2, RotateCw, TriangleAlert, XCircle } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { qk, useTask, useTaskLogs } from "@/lib/queries";
import { api, ApiError } from "@/lib/api";
import { canCancelTask, canRetryTask } from "@/lib/task-rules";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { TaskTypeBadge, TaskTriggerBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
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
import { absoluteUtc } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

function GatedButton({
  enabled,
  reason,
  onClick,
  className,
  children,
}: {
  enabled: boolean;
  reason: string;
  onClick?: () => void;
  className?: string;
  children: React.ReactNode;
}) {
  if (enabled) {
    return (
      <Button variant="outline" className={className} onClick={onClick}>
        {children}
      </Button>
    );
  }
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0}>
          <Button variant="outline" className={className} disabled>
            {children}
          </Button>
        </span>
      </TooltipTrigger>
      <TooltipContent>{reason}</TooltipContent>
    </Tooltip>
  );
}

function InfoRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[110px_1fr] items-start gap-2 py-1.5 text-sm">
      <span className="pt-0.5 text-muted-foreground">{label}</span>
      <span className="min-w-0">{children}</span>
    </div>
  );
}

function TaskLink({ id, label }: { id: string; label?: string }) {
  const navigate = useNavigate();
  return (
    <Button
      variant="link"
      size="sm"
      className="h-auto p-0"
      onClick={() => navigate(`/tasks/${id}`)}
    >
      <Mono className="text-[13px]">{label ?? id}</Mono>
    </Button>
  );
}

const LEVEL_CLASS: Record<string, string> = {
  error: "text-danger",
  warn: "text-warning-muted-foreground",
  info: "text-muted-foreground",
};

export function TaskDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { data, isLoading, isError, error, refetch } = useTask(id);
  const logs = useTaskLogs(id);
  const [confirm, setConfirm] = useState<"retry" | "cancel" | null>(null);
  const [busy, setBusy] = useState(false);

  const runAction = async (kind: "retry" | "cancel") => {
    const label = kind === "retry" ? "重试" : "取消";
    setBusy(true);
    try {
      await api.post(`/tasks/${id}/${kind}`);
      toast.success(`${label}已发起`);
      // 任务态变化联动证书态(回退 / 派生),失效相关缓存
      void refetch();
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.dashboard });
      if (data?.certificateId) {
        qc.invalidateQueries({ queryKey: qk.certificate(data.certificateId) });
        qc.invalidateQueries({ queryKey: qk.certificates });
      }
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : `${label}失败`);
    } finally {
      setBusy(false);
      setConfirm(null);
    }
  };

  if (isLoading) {
    return (
      <div className="mx-auto max-w-3xl space-y-4 p-4 sm:p-6">
        <Skeleton className="h-8 w-56" />
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }
  if (isError || !data) {
    const notFound = error instanceof ApiError && error.code === "task_not_found";
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        <PageHeader title="任务详情" crumbs={[{ label: "任务", to: "/tasks" }]} />
        {notFound ? (
          <EmptyState
            Icon={ListChecks}
            title="任务不存在"
            description="该任务可能不存在。"
            action={<Button onClick={() => navigate("/tasks")}>返回任务列表</Button>}
          />
        ) : (
          <ErrorState error={error} onRetry={() => void refetch()} />
        )}
      </div>
    );
  }

  const s = data.status;
  const domains = data.certificate?.domains ?? data.certificateDomains ?? [];
  const primary = domains[0] ?? "任务";
  const logItems = logs.data?.items ?? [];

  return (
    <div className="mx-auto max-w-3xl space-y-5 p-4 sm:p-6">
      <PageHeader
        title={`${primary} · 任务`}
        crumbs={[{ label: "任务", to: "/tasks" }, { label: "详情" }]}
        actions={
          <div className="flex items-center gap-2">
            <GatedButton
              enabled={canRetryTask(s) && !busy}
              reason="仅失败任务可重试"
              onClick={() => setConfirm("retry")}
            >
              <RotateCw />
              重试
            </GatedButton>
            <GatedButton
              enabled={canCancelTask(s) && !busy}
              reason="仅排队 / 执行中任务可取消"
              className="text-danger"
              onClick={() => setConfirm("cancel")}
            >
              <XCircle />
              取消
            </GatedButton>
          </div>
        }
      />

      {data.failureReason && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertTitle>失败原因</AlertTitle>
          <AlertDescription>{data.failureReason}</AlertDescription>
        </Alert>
      )}

      {/* 基本信息 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            基本信息
            <StatusBadge status={s} />
          </CardTitle>
        </CardHeader>
        <CardContent className="pb-6">
          <InfoRow label="类型">
            <TaskTypeBadge type={data.taskType} />
          </InfoRow>
          <InfoRow label="关联证书">
            {data.certificateDeleted ? (
              <Badge variant="neutral">证书已删除</Badge>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <div className="flex flex-wrap gap-1.5">
                  {domains.length > 0 ? (
                    domains.map((d) => <Mono key={d}>{d}</Mono>)
                  ) : (
                    <span className="text-muted-foreground">—</span>
                  )}
                </div>
                {data.certificate && <StatusBadge status={data.certificate.status} />}
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto p-0"
                  onClick={() => navigate(`/certificates/${data.certificateId}`)}
                >
                  查看证书
                </Button>
              </div>
            )}
          </InfoRow>
          <InfoRow label="触发方式">
            <TaskTriggerBadge trigger={data.trigger} />
          </InfoRow>
          <InfoRow label="尝试次数">第 {data.attemptNumber} 次</InfoRow>
          <InfoRow label="入队">
            <Mono>{absoluteUtc(data.queuedAt)}</Mono>
          </InfoRow>
          <InfoRow label="开始">
            {data.startedAt ? (
              <Mono>{absoluteUtc(data.startedAt)}</Mono>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
          <InfoRow label="结束">
            {data.finishedAt ? (
              <Mono>{absoluteUtc(data.finishedAt)}</Mono>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
          <InfoRow label="结果摘要">
            {data.resultSummary ? (
              <span>{data.resultSummary}</span>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
        </CardContent>
      </Card>

      {/* 重试链 */}
      {(data.parentTaskId || data.childTaskIds.length > 0) && (
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">重试链</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 pb-6 text-sm">
            {data.parentTaskId && (
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">前序失败任务</span>
                <TaskLink id={data.parentTaskId} />
              </div>
            )}
            <div className="text-muted-foreground">本任务为第 {data.attemptNumber} 次尝试。</div>
            {data.childTaskIds.length > 0 && (
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-muted-foreground">后继重试任务</span>
                {data.childTaskIds.map((cid) => (
                  <TaskLink key={cid} id={cid} />
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* 执行日志(mono、脱敏) */}
      <Card className="gap-0 overflow-hidden py-0">
        <CardHeader className="border-b border-border py-4">
          <CardTitle className="text-sm">执行日志</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {logs.isLoading ? (
            <div className="space-y-2 p-4">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-5/6" />
            </div>
          ) : logs.isError ? (
            <div className="p-4">
              <ErrorState error={logs.error} onRetry={() => void logs.refetch()} />
            </div>
          ) : logItems.length === 0 ? (
            <EmptyState
              Icon={ListChecks}
              title="暂无日志"
              description="任务尚未产生执行日志(排队中或执行器为里程碑桩)。"
            />
          ) : (
            <div className="max-h-80 overflow-auto p-3 font-mono text-[12px] leading-relaxed">
              {logItems.map((l) => (
                <div key={l.id} className="flex gap-2 whitespace-pre-wrap break-all py-0.5">
                  <span className="shrink-0 text-muted-foreground">{absoluteUtc(l.loggedAt)}</span>
                  <span className={"shrink-0 uppercase " + (LEVEL_CLASS[l.level] ?? "text-muted-foreground")}>
                    {l.level}
                  </span>
                  <span className="min-w-0">{l.message}</span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* 重试 / 取消二次确认 */}
      <AlertDialog open={!!confirm} onOpenChange={(o) => !o && setConfirm(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {confirm === "retry" ? "重试该任务?" : "取消该任务?"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {confirm === "retry"
                ? "将派生一个新任务(同类型、同证书)入队,原失败任务保留于历史;并驱动关联证书从告警态回到进行中态。"
                : "将取消排队 / 执行中的任务。执行中取消为尽力而为——已提交至 CA 的在途操作可能仍生效,由证书模块下次扫描据实校正。"}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={busy}>返回</AlertDialogCancel>
            <AlertDialogAction
              className={
                confirm === "cancel"
                  ? "bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  : undefined
              }
              disabled={busy}
              onClick={(e) => {
                e.preventDefault();
                if (confirm) void runAction(confirm);
              }}
            >
              {busy && <Loader2 className="animate-spin" />}
              {confirm === "retry" ? "重试" : "取消任务"}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
