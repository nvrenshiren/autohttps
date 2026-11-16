/**
 * 域名详情(domains/detail PRD)—— 基本信息 + 编辑(分组/备注/验证方式,PATCH)+ 关联证书投影 + 删除。
 */
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { ArrowUpRight, Globe, Loader2, Pencil, ShieldCheck, Trash2 } from "lucide-react";
import { useDeleteDomain, useDomain, useUpdateDomain } from "@/lib/queries";
import type { ValidationMethod } from "@/bindings";
import { ApiError } from "@/lib/api";
import { PageHeader } from "@/components/shared/page-header";
import { ErrorState, EmptyState } from "@/components/shared/states";
import { StatusBadge, statusLabel } from "@/components/status-badge";
import { IssuanceMethodBadge, ValidationMethodBadge, WildcardBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
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
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

const NONE = "__none__";

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
      <div className="mx-auto w-full max-w-5xl space-y-4 p-4 sm:p-6 lg:p-8">
        <Skeleton className="h-8 w-56" />
        <Skeleton className="h-52 rounded-xl" />
      </div>
    );
  }
  if (isError || !data) {
    const notFound = error instanceof ApiError && error.code === "domain_not_found";
    return (
      <div className="mx-auto w-full max-w-5xl p-4 sm:p-6 lg:p-8">
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
    <div className="mx-auto w-full max-w-[1440px] space-y-5 p-4 sm:p-6 lg:p-8">
      <PageHeader
        title={data.hostname}
        crumbs={[{ label: "域名", to: "/domains" }, { label: data.hostname }]}
        actions={
          <>
            <Button asChild>
              <Link to={`/certificates/issue?domainId=${encodeURIComponent(data.id)}`} aria-label={`为 ${data.hostname} 发起证书签发`}>
                <ShieldCheck />
                为此域名签发
              </Link>
            </Button>
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

      <div className="grid items-start gap-5 xl:grid-cols-[minmax(280px,0.78fr)_minmax(0,1.22fr)]">
        <section aria-labelledby="domain-profile-title" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
          <div className="flex items-center gap-3 border-b border-border px-5 py-4">
            <span className="inline-flex size-9 items-center justify-center rounded-md bg-accent text-primary"><Globe aria-hidden="true" className="size-[18px]" /></span>
            <div>
              <h2 id="domain-profile-title" className="text-sm font-semibold">域名资料</h2>
              <p className="mt-0.5 text-xs text-muted-foreground">组织方式与验证偏好</p>
            </div>
          </div>
          <dl className="grid gap-x-5 gap-y-5 p-5 sm:grid-cols-2 xl:grid-cols-1">
            <div className="min-w-0">
              <dt className="text-xs font-medium text-muted-foreground">主机名</dt>
              <dd className="mt-1.5 flex min-w-0 flex-wrap items-center gap-2 text-sm">
                <Mono className="max-w-full break-all">{data.hostname}</Mono>
                {data.isWildcard && <WildcardBadge />}
              </dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-muted-foreground">分组</dt>
              <dd className="mt-1.5 text-sm">{data.groupName || <span className="text-muted-foreground">未分组</span>}</dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-muted-foreground">验证方式</dt>
              <dd className="mt-1.5 flex flex-wrap items-center gap-2 text-sm">
                <ValidationMethodBadge method={data.validationMethod} />
                <Link to="/acme" className="rounded-sm text-xs text-link hover:text-link-hover focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring">前往 ACME 配置</Link>
              </dd>
            </div>
            <div>
              <dt className="text-xs font-medium text-muted-foreground">创建时间</dt>
              <dd className="mt-1.5 text-sm">
                <Tooltip><TooltipTrigger asChild><span className="cursor-help text-muted-foreground">{relativeTime(data.createdAt)}</span></TooltipTrigger><TooltipContent><Mono>{absoluteUtc(data.createdAt)}</Mono></TooltipContent></Tooltip>
              </dd>
            </div>
            <div className="sm:col-span-2 xl:col-span-1">
              <dt className="text-xs font-medium text-muted-foreground">备注</dt>
              <dd className="mt-1.5 break-words text-sm leading-relaxed">{data.remark || <span className="text-muted-foreground">暂无备注</span>}</dd>
            </div>
          </dl>
        </section>

        <section aria-labelledby="domain-certificates-title" className="overflow-hidden rounded-xl border border-border bg-card shadow-card">
          <div className="flex min-h-[66px] items-center justify-between gap-3 border-b border-border px-5 py-4">
            <div>
              <h2 id="domain-certificates-title" className="text-sm font-semibold">关联证书</h2>
              <p className="mt-0.5 text-xs text-muted-foreground">{data.certificateCount} 张证书覆盖此域名</p>
            </div>
            <span className="inline-flex size-8 items-center justify-center rounded-md bg-muted text-muted-foreground"><ShieldCheck aria-hidden="true" className="size-4" /></span>
          </div>
          {data.certificates.length === 0 ? (
            <EmptyState Icon={ShieldCheck} title="无关联证书" description="为此域名发起首次签发，证书会显示在这里。" action={<Button asChild size="sm"><Link to={`/certificates/issue?domainId=${encodeURIComponent(data.id)}`}><ShieldCheck />发起签发</Link></Button>} />
          ) : (
            <ul className="divide-y divide-border">
              {data.certificates.map((certificate) => (
                <li key={certificate.id} className="group">
                  <Link
                    to={`/certificates/${certificate.id}`}
                    aria-label={`查看 ${data.hostname} 的证书详情：${certificate.issuanceMethod === "acme" ? "ACME" : "自签"}，${statusLabel(certificate.status)}`}
                    className="flex min-h-[74px] flex-wrap items-center gap-3 px-4 py-3 transition-colors hover:bg-muted/35 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-inset focus-visible:ring-ring sm:px-5"
                  >
                    <span className="min-w-[104px]"><StatusBadge status={certificate.status} /></span>
                    <span className="flex min-w-[94px] items-center gap-1.5"><IssuanceMethodBadge method={certificate.issuanceMethod} /></span>
                    <span className="ml-auto flex min-w-[110px] items-center justify-end gap-1.5 text-xs text-muted-foreground">
                      {certificate.notAfter ? <Tooltip><TooltipTrigger asChild><span className="cursor-help">{daysLabel(certificate.daysUntilExpiry)}</span></TooltipTrigger><TooltipContent><Mono>{absoluteUtc(certificate.notAfter)}</Mono></TooltipContent></Tooltip> : <span>尚未签发</span>}
                      <ArrowUpRight aria-hidden="true" className="size-4 transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5" />
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

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
className="bg-destructive-solid text-destructive-foreground hover:bg-destructive-hover active:bg-destructive-active"
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
