/**
 * 域名详情(domains/detail PRD)—— 基本信息 + 编辑(分组/备注/验证方式,PATCH)+ 关联证书投影 + 删除。
 */
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { Globe, Loader2, Pencil, ShieldCheck, Trash2 } from "lucide-react";
import { useDeleteDomain, useDomain, useUpdateDomain } from "@/lib/queries";
import type { ValidationMethod } from "@/bindings";
import { ApiError } from "@/lib/api";
import { PageHeader } from "@/components/shared/page-header";
import { ErrorState, EmptyState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { IssuanceMethodBadge, ValidationMethodBadge, WildcardBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

const NONE = "__none__";

function InfoRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[110px_1fr] items-center gap-2 py-1.5 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="min-w-0">{children}</span>
    </div>
  );
}

export function DomainDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { data, isLoading, isError, error, refetch } = useDomain(id);
  const update = useUpdateDomain(id);
  const del = useDeleteDomain();

  const [editOpen, setEditOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [groupName, setGroupName] = useState("");
  const [remark, setRemark] = useState("");
  const [validationMethod, setValidationMethod] = useState<string>(NONE);

  useEffect(() => {
    if (data && editOpen) {
      setGroupName(data.groupName ?? "");
      setRemark(data.remark ?? "");
      setValidationMethod(data.validationMethod ?? NONE);
    }
  }, [data, editOpen]);

  if (isLoading) {
    return (
      <div className="mx-auto max-w-3xl space-y-4 p-4 sm:p-6">
        <Skeleton className="h-8 w-56" />
        <Skeleton className="h-52 rounded-xl" />
      </div>
    );
  }
  if (isError || !data) {
    const notFound = error instanceof ApiError && error.code === "domain_not_found";
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        <PageHeader title="域名详情" crumbs={[{ label: "域名", to: "/domains" }]} />
        {notFound ? (
          <EmptyState
            Icon={Globe}
            title="域名不存在"
            description="该域名可能已被删除。"
            action={<Button onClick={() => navigate("/domains")}>返回域名列表</Button>}
          />
        ) : (
          <ErrorState error={error} onRetry={() => void refetch()} />
        )}
      </div>
    );
  }

  const blockedDelete = data.certificateCount > 0;

  const onSaveEdit = () => {
    update.mutate(
      {
        groupName: groupName.trim() === "" ? null : groupName.trim(),
        remark: remark.trim() === "" ? null : remark.trim(),
        validationMethod: validationMethod === NONE ? null : (validationMethod as ValidationMethod),
      },
      {
        onSuccess: () => {
          toast.success("已保存");
          setEditOpen(false);
        },
        onError: (e) => toast.error(e instanceof ApiError ? e.message : "保存失败"),
      },
    );
  };

  const onDelete = () => {
    del.mutate(data.id, {
      onSuccess: () => {
        toast.success(`已删除域名 ${data.hostname}`);
        navigate("/domains");
      },
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "删除失败"),
    });
  };

  return (
    <div className="mx-auto max-w-3xl space-y-5 p-4 sm:p-6">
      <PageHeader
        title={data.hostname}
        crumbs={[{ label: "域名", to: "/domains" }, { label: data.hostname }]}
        actions={
          <>
            <Button variant="secondary" onClick={() => setEditOpen(true)}>
              <Pencil />
              编辑
            </Button>
            {blockedDelete ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <span>
                    <Button variant="outline" className="text-danger" disabled>
                      <Trash2 />
                      删除
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>被 {data.certificateCount} 个证书关联,不可删除</TooltipContent>
              </Tooltip>
            ) : (
              <Button variant="outline" className="text-danger" onClick={() => setConfirmDelete(true)}>
                <Trash2 />
                删除
              </Button>
            )}
          </>
        }
      />

      {/* 基本信息 */}
      <Card>
        <CardHeader>
          <CardTitle>基本信息</CardTitle>
        </CardHeader>
        <CardContent className="pb-6">
          <InfoRow label="hostname">
            <div className="flex items-center gap-2">
              <Mono>{data.hostname}</Mono>
              {data.isWildcard && <WildcardBadge />}
            </div>
          </InfoRow>
          <InfoRow label="分组">
            {data.groupName ?? <span className="text-muted-foreground">未分组</span>}
          </InfoRow>
          <InfoRow label="验证方式">
            <ValidationMethodBadge method={data.validationMethod} />
          </InfoRow>
          <InfoRow label="备注">
            {data.remark ?? <span className="text-muted-foreground">—</span>}
          </InfoRow>
          <InfoRow label="创建">
            <Tooltip>
              <TooltipTrigger asChild>
                <span className="cursor-help text-muted-foreground">
                  {relativeTime(data.createdAt)}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                <Mono>{absoluteUtc(data.createdAt)}</Mono>
              </TooltipContent>
            </Tooltip>
          </InfoRow>
        </CardContent>
      </Card>

      {/* 关联证书投影 */}
      <Card className="gap-0 overflow-hidden py-0">
        <CardHeader className="border-b border-border py-4">
          <CardTitle>关联证书({data.certificateCount})</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          {data.certificates.length === 0 ? (
            <EmptyState
              Icon={ShieldCheck}
              title="无关联证书"
              description="该域名尚未被任何证书关联。"
            />
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>状态</TableHead>
                  <TableHead>签发方式</TableHead>
                  <TableHead>到期</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.certificates.map((c) => (
                  <TableRow
                    key={c.id}
                    className="cursor-pointer"
                    onClick={() => navigate(`/certificates/${c.id}`)}
                  >
                    <TableCell>
                      <StatusBadge status={c.status} />
                    </TableCell>
                    <TableCell>
                      <IssuanceMethodBadge method={c.issuanceMethod} />
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {daysLabel(c.daysUntilExpiry)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* 编辑对话框 */}
      <Dialog open={editOpen} onOpenChange={setEditOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>编辑域名</DialogTitle>
            <DialogDescription>hostname 不可修改;可调整分组 / 备注 / 验证方式。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label>hostname</Label>
              <div className="rounded-md border border-border bg-muted/50 px-3 py-2">
                <Mono>{data.hostname}</Mono>
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="edit-group">分组</Label>
              <Input
                id="edit-group"
                value={groupName}
                onChange={(e) => setGroupName(e.target.value)}
                placeholder="留空以清除分组"
              />
            </div>
            <div className="space-y-1.5">
              <Label>验证方式</Label>
              <Select value={validationMethod} onValueChange={setValidationMethod}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NONE}>未设置</SelectItem>
                  <SelectItem value="http_01">HTTP-01(webroot)</SelectItem>
                  <SelectItem value="dns_01">DNS-01(手动)</SelectItem>
                </SelectContent>
              </Select>
              {data.isWildcard && (
                <p className="text-xs text-muted-foreground">通配符域名只能使用 DNS-01。</p>
              )}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="edit-remark">备注</Label>
              <Input
                id="edit-remark"
                value={remark}
                onChange={(e) => setRemark(e.target.value)}
                placeholder="留空以清除备注"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditOpen(false)} disabled={update.isPending}>
              取消
            </Button>
            <Button onClick={onSaveEdit} disabled={update.isPending}>
              {update.isPending && <Loader2 className="animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认 */}
      <AlertDialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>删除域名?</AlertDialogTitle>
            <AlertDialogDescription>
              将删除 <Mono>{data.hostname}</Mono> 及其元数据。此操作不可撤销。
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
