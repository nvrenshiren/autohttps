/**
 * 证书详情(certificates/detail PRD)—— 信息 + 生命周期操作(按状态启用/禁用 + Tooltip 原因,H4;
 * 破坏性走 AlertDialog,H5)。续签 / 重试 / 吊销 / 删除接真端点;导出经内容选择面板(私钥走风险确认 H6)
 * + lib/download 二进制下载。
 */
import { useState } from "react";
import { useNavigate, useParams } from "react-router";
import {
  Ban,
  Download,
  KeyRound,
  ListChecks,
  Loader2,
  RotateCw,
  ShieldCheck,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import { useCertificate, useDeleteCertificate } from "@/lib/queries";
import type { CertificateStatus } from "@/bindings";
import { api, ApiError } from "@/lib/api";
import { downloadFile } from "@/lib/download";
import { canDelete, canRenew, canRetry, canRevoke, isExportable, isInProgress } from "@/lib/cert-rules";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { IssuanceMethodBadge, WildcardBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";
import { useQueryClient } from "@tanstack/react-query";
import { qk } from "@/lib/queries";

/** 导出内容预设(§2.8 parts 词表;非 §4.3 状态枚举,可用字面量)。 */
const EXPORT_PRESETS: { value: string; label: string; hasKey: boolean }[] = [
  { value: "fullchain", label: "完整证书链(叶子 + 链)", hasKey: false },
  { value: "leaf", label: "仅叶子证书", hasKey: false },
  { value: "chain", label: "仅证书链(签发根 CA)", hasKey: false },
  { value: "private_key", label: "仅私钥(敏感)", hasKey: true },
  { value: "fullchain,private_key", label: "完整链 + 私钥(部署用,敏感)", hasKey: true },
];

function GatedButton({
  enabled,
  reason,
  onClick,
  variant = "outline",
  className,
  children,
}: {
  enabled: boolean;
  reason: string;
  onClick?: () => void;
  variant?: "outline" | "secondary" | "default";
  className?: string;
  children: React.ReactNode;
}) {
  if (enabled) {
    return (
      <Button variant={variant} className={className} onClick={onClick}>
        {children}
      </Button>
    );
  }
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span tabIndex={0}>
          <Button variant={variant} className={className} disabled>
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

export function CertificateDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { data, isLoading, isError, error, refetch } = useCertificate(id);
  const del = useDeleteCertificate();
  const [confirmRevoke, setConfirmRevoke] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const [exportParts, setExportParts] = useState("fullchain");
  const [keyAck, setKeyAck] = useState(false);
  const [busy, setBusy] = useState(false);

  const runAction = async (path: string, label: string) => {
    setBusy(true);
    try {
      await api.post(path);
      toast.success(`${label}已发起`);
      qc.invalidateQueries({ queryKey: qk.certificate(id) });
      qc.invalidateQueries({ queryKey: qk.certificates });
      qc.invalidateQueries({ queryKey: qk.tasks });
      qc.invalidateQueries({ queryKey: qk.dashboard });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : `${label}失败`);
    } finally {
      setBusy(false);
    }
  };

  const openExport = () => {
    setExportParts("fullchain");
    setKeyAck(false);
    setExportOpen(true);
  };

  const needsKeyAck = EXPORT_PRESETS.find((p) => p.value === exportParts)?.hasKey ?? false;

  const onDownload = async () => {
    const primary = data?.domains[0]?.hostname?.replace(/[^a-zA-Z0-9.-]/g, "_") ?? id;
    const suffix = exportParts.replace(/,/g, "+");
    const q = `parts=${exportParts}${needsKeyAck ? "&acknowledgeKeyExport=true" : ""}`;
    setBusy(true);
    try {
      await downloadFile(`/certificates/${id}/export?${q}`, `${primary}-${suffix}.pem`);
      toast.success("导出完成");
      setExportOpen(false);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "导出失败");
    } finally {
      setBusy(false);
    }
  };

  const onDelete = () => {
    del.mutate(id, {
      onSuccess: () => {
        toast.success("证书已删除");
        navigate("/certificates");
      },
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "删除失败"),
    });
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
    const notFound = error instanceof ApiError && error.code === "cert_not_found";
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        <PageHeader title="证书详情" crumbs={[{ label: "证书", to: "/certificates" }]} />
        {notFound ? (
          <EmptyState
            Icon={ShieldCheck}
            title="证书不存在"
            description="该证书可能已被删除。"
            action={<Button onClick={() => navigate("/certificates")}>返回证书列表</Button>}
          />
        ) : (
          <ErrorState error={error} onRetry={() => void refetch()} />
        )}
      </div>
    );
  }

  const s: CertificateStatus = data.status;
  const primary = data.domains[0]?.hostname ?? "证书详情";
  const inProgress = isInProgress(s);

  return (
    <div className="mx-auto max-w-3xl space-y-5 p-4 sm:p-6">
      <PageHeader
        title={primary}
        crumbs={[{ label: "证书", to: "/certificates" }, { label: primary }]}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={() => navigate(`/tasks?certificateId=${id}`)}
            >
              <ListChecks />
              关联任务
            </Button>
            <GatedButton
              enabled={canRenew(s) && !busy}
              reason="仅有效 / 即将到期 / 已过期 / 已吊销证书可续签"
              onClick={() => runAction(`/certificates/${id}/renew`, "续签")}
            >
              <RotateCw />
              续签
            </GatedButton>
            {canRetry(s) && (
              <GatedButton
                enabled={!busy}
                reason=""
                onClick={() => runAction(`/certificates/${id}/retry`, "重试")}
              >
                <RotateCw />
                重试
              </GatedButton>
            )}
            <GatedButton
              enabled={isExportable(s) && !busy}
              reason="尚无本地证书文件,不可导出"
              onClick={openExport}
            >
              <Download />
              导出
            </GatedButton>
            <GatedButton
              enabled={canRevoke(s) && !busy}
              reason="仅有效 / 即将到期 / 续签失败证书可吊销"
              className="text-danger"
              onClick={() => setConfirmRevoke(true)}
            >
              <Ban />
              吊销
            </GatedButton>
            <GatedButton
              enabled={canDelete(s) && !del.isPending}
              reason="进行中态不可删除,请先取消其任务"
              className="text-danger"
              onClick={() => setConfirmDelete(true)}
            >
              <Trash2 />
              删除
            </GatedButton>
          </div>
        }
      />

      {inProgress && (
        <Alert>
          <Loader2 className="animate-spin" />
          <AlertTitle>操作进行中</AlertTitle>
          <AlertDescription>
            当前有进行中的任务
            {data.activeTaskId ? (
              <>
                (
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto p-0"
                  onClick={() => navigate(`/tasks/${data.activeTaskId}`)}
                >
                  <Mono>{data.activeTaskId}</Mono>
                </Button>
                )
              </>
            ) : (
              " "
            )}
            。取消进行中操作请前往该任务详情发起。
          </AlertDescription>
        </Alert>
      )}

      {data.lastError && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertTitle>最近失败原因</AlertTitle>
          <AlertDescription>{data.lastError}</AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            基本信息
            <StatusBadge status={s} />
          </CardTitle>
        </CardHeader>
        <CardContent className="pb-6">
          <InfoRow label="域名(SAN)">
            <div className="flex flex-wrap gap-1.5">
              {data.domains.map((d) => (
                <span
                  key={d.id}
                  className="inline-flex items-center gap-1 rounded-md border border-border bg-muted/40 px-1.5 py-0.5"
                >
                  <Mono>{d.hostname}</Mono>
                  {d.isWildcard && <WildcardBadge />}
                </span>
              ))}
            </div>
          </InfoRow>
          <InfoRow label="签发方式">
            <div className="flex items-center gap-2">
              <IssuanceMethodBadge method={data.issuanceMethod} />
              {data.acmeAccount && (
                <span className="text-xs text-muted-foreground">
                  账户 {data.acmeAccount.caLabel ?? data.acmeAccount.id}
                </span>
              )}
              {data.rootCa && (
                <span className="text-xs text-muted-foreground">根 CA {data.rootCa.name}</span>
              )}
            </div>
          </InfoRow>
          <InfoRow label="序列号">
            {data.serialNumber ? <Mono>{data.serialNumber}</Mono> : <span className="text-muted-foreground">—</span>}
          </InfoRow>
          <InfoRow label="指纹">
            {data.fingerprint ? (
              <Mono className="break-all">{data.fingerprint}</Mono>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
          <InfoRow label="有效期">
            {data.notAfter ? (
              <div className="space-y-0.5">
                <div>
                  <Mono>{absoluteUtc(data.notBefore)}</Mono>
                  <span className="mx-1 text-muted-foreground">→</span>
                  <Mono>{absoluteUtc(data.notAfter)}</Mono>
                </div>
                <div className="text-xs text-muted-foreground">{daysLabel(data.daysUntilExpiry)}</div>
              </div>
            ) : (
              <span className="text-muted-foreground">未签发</span>
            )}
          </InfoRow>
          <InfoRow label="签发时间">
            {data.issuedAt ? <Mono>{absoluteUtc(data.issuedAt)}</Mono> : <span className="text-muted-foreground">—</span>}
          </InfoRow>
          <InfoRow label="创建 / 更新">
            <span className="text-muted-foreground">
              {relativeTime(data.createdAt)} · 更新 {relativeTime(data.updatedAt)}
            </span>
          </InfoRow>
        </CardContent>
      </Card>

      {/* 吊销确认 */}
      <AlertDialog open={confirmRevoke} onOpenChange={setConfirmRevoke}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>吊销该证书?</AlertDialogTitle>
            <AlertDialogDescription>
              吊销后证书将不可用。ACME 证书会向 CA 发起吊销;自签证书在根 CA 名下记作废。此操作不可撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={(e) => {
                e.preventDefault();
                setConfirmRevoke(false);
                void runAction(`/certificates/${id}/revoke`, "吊销");
              }}
            >
              吊销
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* 删除确认 */}
      <AlertDialog open={confirmDelete} onOpenChange={setConfirmDelete}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>删除该证书?</AlertDialogTitle>
            <AlertDialogDescription>
              将删除证书条目及本地证书 / 私钥文件,并取消其未完成任务;历史任务只读保留。此操作不可撤销。
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

      {/* 导出内容选择面板(H6:含私钥走风险确认) */}
      <Dialog open={exportOpen} onOpenChange={(o) => !busy && setExportOpen(o)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>导出证书</DialogTitle>
            <DialogDescription>
              选择导出内容;PEM 文件下载(服务器形态经浏览器下载,桌面形态保存到本地路径)。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="export-parts">导出内容</Label>
              <Select
                value={exportParts}
                onValueChange={(v) => {
                  setExportParts(v);
                  setKeyAck(false);
                }}
              >
                <SelectTrigger id="export-parts" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {EXPORT_PRESETS.map((p) => (
                    <SelectItem key={p.value} value={p.value}>
                      {p.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {needsKeyAck && (
              <Alert variant="destructive">
                <KeyRound />
                <AlertTitle>私钥属敏感数据</AlertTitle>
                <AlertDescription>
                  <p>
                    导出的私钥可解密该证书的全部流量,请务必妥善保管、切勿外泄或经不安全渠道传输。
                  </p>
                  <div className="mt-2 flex items-center gap-2">
                    <Switch id="key-ack" checked={keyAck} onCheckedChange={setKeyAck} />
                    <Label htmlFor="key-ack" className="cursor-pointer font-normal">
                      我已知晓风险,确认导出私钥
                    </Label>
                  </div>
                </AlertDescription>
              </Alert>
            )}
          </div>
          <DialogFooter>
            <Button variant="secondary" disabled={busy} onClick={() => setExportOpen(false)}>
              取消
            </Button>
            <Button disabled={busy || (needsKeyAck && !keyAck)} onClick={() => void onDownload()}>
              {busy && <Loader2 className="animate-spin" />}
              <Download />
              下载
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
