/**
 * 交互态组件(设计 §7.9 / §10-H3):loading(Skeleton)/ empty(Empty 三语气)/ error(Alert + 重试)。
 */
import { CircleAlert, RotateCw, type LucideIcon } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { TableCell, TableRow } from "@/components/ui/table";
import { ApiError } from "@/lib/api";
import { cn } from "@/lib/utils";

export function EmptyState({
  Icon,
  title,
  description,
  action,
  iconClassName,
}: {
  Icon: LucideIcon;
  title: string;
  description?: string;
  action?: React.ReactNode;
  iconClassName?: string;
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-2.5 px-6 py-16 text-center">
      <span className="inline-flex size-12 items-center justify-center rounded-2xl border border-border bg-muted/50 shadow-xs">
        <Icon className={cn("size-6 text-muted-foreground", iconClassName)} />
      </span>
      <div className="text-sm font-medium text-foreground">{title}</div>
      {description && (
        <div className="max-w-sm text-[13px] leading-relaxed text-muted-foreground">
          {description}
        </div>
      )}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const message =
    error instanceof ApiError
      ? error.message
      : error instanceof Error
        ? error.message
        : "加载失败,请稍后重试";
  return (
    <Alert variant="destructive">
      <CircleAlert />
      <AlertTitle>加载失败</AlertTitle>
      <AlertDescription>
        <p>{message}</p>
        {onRetry && (
          <Button size="sm" variant="outline" className="mt-1 w-fit" onClick={onRetry}>
            <RotateCw />
            重试
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}

export function TableSkeletonRows({ rows = 6, cols }: { rows?: number; cols: number }) {
  return (
    <>
      {Array.from({ length: rows }).map((_, r) => (
        <TableRow key={r}>
          {Array.from({ length: cols }).map((__, c) => (
            <TableCell key={c}>
              <Skeleton className="h-4 w-full max-w-[140px]" />
            </TableCell>
          ))}
        </TableRow>
      ))}
    </>
  );
}
