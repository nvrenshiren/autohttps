/**
 * 根 CA 详情(local-ca/detail PRD)—— 完整信息 + 导出公开证书(§2.4 两态均可)+ 内网证书概览(计数,
 * 明细归 certificates)+ 已过期处置引导(warning Alert,H6)。无显式移除(LC5)。
 *
 * 注:F2「逐条跳内网证书」需 `GET /certificates?rootCaId=` 过滤(后端 CertListQuery 暂未支持),本切片
 * 按任务口径呈现 issuedCertificateCount 概览 + 跳证书列表;逐条明细待该过滤上线。
 */
import { useState } from "react";
import { useNavigate, useParams } from "react-router";
import { Check, Copy, Download, Landmark, Plus, ShieldCheck } from "lucide-react";
import { useRootCa } from "@/lib/queries";
import { ApiError } from "@/lib/api";
import { downloadFile } from "@/lib/download";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { CreationMethodBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { absoluteUtc, daysLabel, relativeTime } from "@/lib/time";
import { toast } from "@/components/ui/sonner";

function InfoRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[110px_1fr] items-start gap-2 py-1.5 text-sm">
      <span className="pt-0.5 text-muted-foreground">{label}</span>
      <span className="min-w-0">{children}</span>
    </div>
  );
}

export function RootCaDetailPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { data, isLoading, isError, error, refetch } = useRootCa(id);
  const [exporting, setExporting] = useState(false);
  const [copied, setCopied] = useState(false);

  const onExport = async () => {
    if (!data) return;
    setExporting(true);
    try {
      await downloadFile(`/root-cas/${data.id}/export`, `root-ca-${data.name || data.id}.pem`);
      toast.success("已导出根 CA 证书");
    } catch (e) {
      toast.error(e instanceof ApiError || e instanceof Error ? e.message : "导出失败");
    } finally {
      setExporting(false);
    }
  };

  const onCopyPem = async () => {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(data.certPem);
      setCopied(true);
      toast.success("已复制证书 PEM");
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("复制失败");
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
    const notFound = error instanceof ApiError && error.code === "root_ca_not_found";
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        <PageHeader title="根 CA 详情" crumbs={[{ label: "根 CA", to: "/local-ca" }]} />
        {notFound ? (
          <EmptyState
            Icon={Landmark}
            title="根 CA 不存在"
            description="该根 CA 可能不存在或已失效。"
            action={<Button onClick={() => navigate("/local-ca")}>返回根 CA 列表</Button>}
          />
        ) : (
          <ErrorState error={error} onRetry={() => void refetch()} />
        )}
      </div>
    );
  }

  const expired = data.status === "expired";

  return (
    <div className="mx-auto max-w-3xl space-y-5 p-4 sm:p-6">
      <PageHeader
        title={data.name}
        crumbs={[{ label: "根 CA", to: "/local-ca" }, { label: data.name }]}
        actions={
          <Button variant="outline" disabled={exporting} onClick={() => void onExport()}>
            <Download />
            导出证书
          </Button>
        }
      />

      {expired && (
        <Alert variant="warning">
          <ShieldCheck />
          <AlertTitle>此根 CA 已过期</AlertTitle>
          <AlertDescription>
            <p>已过期的根 CA 不可再用于签发新内网证书。请创建或导入新的根 CA 接替签发;此前签出的内网证书随自身有效期继续由证书状态机管理。</p>
            <Button
              size="sm"
              variant="outline"
              className="mt-1 w-fit"
              onClick={() => navigate("/local-ca/new")}
            >
              <Plus />
              新增根 CA
            </Button>
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            基本信息
            <StatusBadge status={data.status} />
          </CardTitle>
        </CardHeader>
        <CardContent className="pb-6">
          <InfoRow label="名称">
            <span className="font-medium">{data.name}</span>
          </InfoRow>
          <InfoRow label="创建方式">
            <CreationMethodBadge method={data.creationMethod} />
          </InfoRow>
          <InfoRow label="有效期">
            <div className="space-y-0.5">
              <div>
                <Mono>{absoluteUtc(data.notBefore)}</Mono>
                <span className="mx-1 text-muted-foreground">→</span>
                <Mono>{absoluteUtc(data.notAfter)}</Mono>
              </div>
              <div className="text-xs text-muted-foreground">{daysLabel(data.daysUntilExpiry)}</div>
            </div>
          </InfoRow>
          <InfoRow label="序列号">
            {data.serialNumber ? (
              <Mono className="break-all">{data.serialNumber}</Mono>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
          <InfoRow label="指纹">
            {data.fingerprint ? (
              <Mono className="break-all">{data.fingerprint}</Mono>
            ) : (
              <span className="text-muted-foreground">—</span>
            )}
          </InfoRow>
          <InfoRow label="内网证书">
            <div className="flex items-center gap-2">
              <span>
                <span className="font-medium">{data.issuedCertificateCount}</span> 张由本根 CA 签发
              </span>
              {data.issuedCertificateCount > 0 && (
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto p-0"
                  onClick={() => navigate("/certificates")}
                >
                  前往证书
                </Button>
              )}
            </div>
          </InfoRow>
          <InfoRow label="创建时间">
            <span className="text-muted-foreground">{relativeTime(data.createdAt)}</span>
          </InfoRow>
        </CardContent>
      </Card>

      {/* 公开证书 PEM(可复制;不含私钥,LC4) */}
      <Card className="gap-0 overflow-hidden py-0">
        <CardHeader className="flex-row items-center justify-between border-b border-border py-4">
          <CardTitle className="text-sm">根 CA 证书(PEM · 公开)</CardTitle>
          <Button variant="ghost" size="sm" onClick={() => void onCopyPem()}>
            {copied ? <Check className="text-success" /> : <Copy />}
            复制
          </Button>
        </CardHeader>
        <CardContent className="p-0">
          <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-all bg-muted/40 p-4 font-mono text-[12px] leading-relaxed">
            {data.certPem}
          </pre>
        </CardContent>
      </Card>
    </div>
  );
}
