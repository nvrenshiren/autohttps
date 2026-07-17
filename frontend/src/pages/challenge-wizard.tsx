/**
 * 验证方式向导页(acme/challenge-wizard PRD)—— 按 certificateId 展示本次签发 / 续签各域名的挑战时间线
 * (§7.11 步骤,禁假进度条)。DNS-01「等待手动配置」展示待加 TXT(名/值 mono + 复制,§7.10)+「我已添加,
 * 去校验」→ confirm;HTTP-01 展示自动进度;失败可重试。多域名整体判定(flows/acme §3.4)。SSE 实时刷新。
 * 首签 / 续签无差异(本页不感知,DEA1);从证书详情「查看验证」或签发后进入。
 */
import { useNavigate, useParams } from "react-router";
import {
  BadgeCheck,
  CircleCheckBig,
  CircleX,
  Hourglass,
  Landmark,
  ListChecks,
  Loader2,
  LoaderCircle,
  RotateCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import {
  useCertificate,
  useChallenge,
  useChallenges,
  useConfirmChallenge,
  useRetryChallenge,
} from "@/lib/queries";
import type { ChallengeStatus, ChallengeSummary, ValidationMethod } from "@/bindings";
import { ApiError } from "@/lib/api";
import { isInProgress } from "@/lib/cert-rules";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { ValidationMethodBadge } from "@/components/shared/category-badges";
import { Mono, CopyableValue } from "@/components/shared/mono";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { toast } from "@/components/ui/sonner";

// ---------- 挑战时间线(§7.11:步骤 + §3.2 状态图标;禁假进度条)----------

interface Phase {
  status: ChallengeStatus;
  label: string;
}

function phasesFor(method: ValidationMethod): Phase[] {
  if (method === "dns_01") {
    return [
      { status: "pending", label: "待验证" },
      { status: "awaiting_manual", label: "等待手动配置" },
      { status: "validating", label: "验证中" },
      { status: "passed", label: "验证通过" },
    ];
  }
  return [
    { status: "pending", label: "待验证" },
    { status: "validating", label: "验证中" },
    { status: "passed", label: "验证通过" },
  ];
}

/** 当前所处步骤;失败 / 已取消落到末节点(该节点直接呈现真实终态 StatusBadge)。 */
function currentPhaseIndex(phases: Phase[], status: ChallengeStatus): number {
  if (status === "failed" || status === "cancelled") return phases.length - 1;
  const i = phases.findIndex((p) => p.status === status);
  return i < 0 ? 0 : i;
}

function ChallengeTimeline({
  method,
  status,
}: {
  method: ValidationMethod;
  status: ChallengeStatus;
}) {
  const phases = phasesFor(method);
  const cur = currentPhaseIndex(phases, status);
  return (
    <ol className="space-y-0">
      {phases.map((ph, i) => {
        const isCurrent = i === cur;
        const reached = i < cur;
        const last = i === phases.length - 1;
        return (
          <li key={ph.status + i} className="flex gap-2.5">
            <div className="flex flex-col items-center">
              <span
                className={cn(
                  "mt-1.5 inline-flex size-2 shrink-0 rounded-full",
                  isCurrent
                    ? "bg-primary ring-2 ring-primary/25"
                    : reached
                      ? "bg-primary"
                      : "bg-muted-foreground/30",
                )}
              />
              {!last && (
                <span className={cn("w-px flex-1", reached ? "bg-primary/40" : "bg-border")} />
              )}
            </div>
            <div className={cn("min-w-0 pb-3", last && "pb-0")}>
              {isCurrent ? (
                <StatusBadge status={status} />
              ) : (
                <span
                  className={cn("text-sm", reached ? "text-foreground" : "text-muted-foreground")}
                >
                  {ph.label}
                </span>
              )}
            </div>
          </li>
        );
      })}
    </ol>
  );
}

// ---------- 单域名挑战卡片 ----------

function ChallengeCard({ challenge }: { challenge: ChallengeSummary }) {
  const c = challenge;
  const isDns = c.validationMethod === "dns_01";
  // 仅 DNS-01 需拉详情取 TXT(HTTP-01 自动、摘要即够)。
  const detailQ = useChallenge(c.id, isDns && (c.status === "awaiting_manual" || c.status === "failed"));
  const confirm = useConfirmChallenge();
  const retry = useRetryChallenge();

  const onConfirm = () =>
    confirm.mutate(c.id, {
      onSuccess: () => toast.success("已提交校验,等待 CA 验证"),
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "提交失败"),
    });
  const onRetry = () =>
    retry.mutate(c.id, {
      onSuccess: () => toast.success("已重新发起验证"),
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "重试失败"),
    });

  const hostname = c.domainHostname ?? "(域名已删除)";
  const detail = detailQ.data;

  return (
    <Card className="gap-0 overflow-hidden py-0">
      <CardHeader className="flex-row items-center justify-between gap-2 border-b border-border py-3">
        <div className="flex min-w-0 items-center gap-2">
          <Mono className="truncate font-medium">{hostname}</Mono>
          <ValidationMethodBadge method={c.validationMethod} />
        </div>
        <StatusBadge status={c.status} />
      </CardHeader>
      <CardContent className="space-y-3 py-4">
        <ChallengeTimeline method={c.validationMethod} status={c.status} />

        {/* DNS-01 等待手动配置:展示 TXT + 确认(CT4)*/}
        {isDns && c.status === "awaiting_manual" && (
          <div className="space-y-3 rounded-lg border border-warning/40 bg-warning-muted/25 p-3">
            <div className="flex items-start gap-2 text-sm text-warning-muted-foreground">
              <Hourglass className="mt-0.5 size-4 shrink-0 text-warning" />
              <p>
                在你的 DNS 服务商处为该域名添加下列 TXT 记录,生效后点「我已添加,去校验」;CA 校验通过后可移除。
              </p>
            </div>
            {detailQ.isLoading ? (
              <div className="space-y-2">
                <Skeleton className="h-8 w-full" />
                <Skeleton className="h-8 w-full" />
              </div>
            ) : detail?.dnsTxtName && detail?.dnsTxtValue ? (
              <div className="space-y-2">
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">TXT 记录名</Label>
                  <CopyableValue value={detail.dnsTxtName} />
                </div>
                <div className="space-y-1">
                  <Label className="text-xs text-muted-foreground">TXT 记录值</Label>
                  <CopyableValue value={detail.dnsTxtValue} />
                </div>
              </div>
            ) : null}
            <Button size="sm" onClick={onConfirm} disabled={confirm.isPending}>
              {confirm.isPending ? <Loader2 className="animate-spin" /> : <ShieldCheck />}
              我已添加,去校验
            </Button>
          </div>
        )}

        {/* HTTP-01 自动进度说明 */}
        {!isDns && (c.status === "pending" || c.status === "validating") && (
          <p className="text-xs text-muted-foreground">
            HTTP-01 由系统自动放置验证文件并请求 CA 校验,无需手动介入;完成后本页实时更新。
          </p>
        )}

        {/* 失败:原因概要 + 重试(CT7)*/}
        {c.status === "failed" && (
          <div className="space-y-2">
            <Alert variant="destructive">
              <CircleX />
              <AlertTitle>验证失败</AlertTitle>
              <AlertDescription>
                <p>{c.failedReason ?? "验证未通过。"}</p>
                <p className="text-xs opacity-80">完整失败日志在任务模块查看。</p>
              </AlertDescription>
            </Alert>
            <Button variant="outline" size="sm" onClick={onRetry} disabled={retry.isPending}>
              <RotateCw className={cn(retry.isPending && "animate-spin")} />
              重试
            </Button>
          </div>
        )}

        {/* 通过:DNS-01 提示可移除 TXT(验收8)*/}
        {c.status === "passed" && isDns && (
          <p className="text-xs text-muted-foreground">验证已通过,可移除该域名的 TXT 记录。</p>
        )}
      </CardContent>
    </Card>
  );
}

// ---------- 整体判定横幅(多域名,flows/acme §3.4)----------

function OverallBanner({ challenges }: { challenges: ChallengeSummary[] }) {
  const allPassed = challenges.every((c) => c.status === "passed");
  const anyFailed = challenges.some((c) => c.status === "failed" || c.status === "cancelled");
  const anyAwaiting = challenges.some((c) => c.status === "awaiting_manual");

  let kind: "success" | "danger" | "warning" | "info";
  let title: string;
  let desc: string;
  if (allPassed) {
    kind = "success";
    title = "全部域名验证通过";
    desc = "证书已取得,状态由证书模块管理。";
  } else if (anyFailed) {
    kind = "danger";
    title = "本次验证未通过";
    desc = "有域名验证失败 / 已取消(任一失败即整体失败)。可对失败域名重试。";
  } else if (anyAwaiting) {
    kind = "warning";
    title = "有域名等待手动配置";
    desc = "请为下方 DNS-01 域名添加 TXT 记录后点「我已添加,去校验」。";
  } else {
    kind = "info";
    title = "验证进行中";
    desc = "全部域名验证通过后方可取证;本页随挑战状态实时更新。";
  }

  const meta = {
    success: { Icon: CircleCheckBig, box: "border-success/40 bg-success-muted/25 text-success-muted-foreground", icon: "text-success", spin: false },
    danger: { Icon: TriangleAlert, box: "border-danger/40 bg-danger-muted/25 text-danger-muted-foreground", icon: "text-danger", spin: false },
    warning: { Icon: Hourglass, box: "border-warning/40 bg-warning-muted/25 text-warning-muted-foreground", icon: "text-warning", spin: false },
    info: { Icon: LoaderCircle, box: "border-info/40 bg-info-muted/25 text-info-muted-foreground", icon: "text-info", spin: true },
  }[kind];

  return (
    <div className={cn("flex items-start gap-3 rounded-lg border px-4 py-3 text-sm", meta.box)}>
      <meta.Icon className={cn("mt-0.5 size-4 shrink-0", meta.icon, meta.spin && "animate-spin")} />
      <div className="min-w-0">
        <div className="font-medium">{title}</div>
        <div className="opacity-90">{desc}</div>
      </div>
    </div>
  );
}

// ---------- 页面 ----------

export function ChallengeWizardPage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const certQ = useCertificate(id);
  const chalQ = useChallenges(id);

  const primary = certQ.data?.domains[0]?.hostname ?? "验证方式向导";
  const crumbs = [
    { label: "证书", to: "/certificates" },
    { label: primary, to: `/certificates/${id}` },
    { label: "验证" },
  ];

  const header = (
    <PageHeader
      title="验证方式向导"
      crumbs={crumbs}
      actions={
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" onClick={() => navigate(`/certificates/${id}`)}>
            <ShieldCheck />
            证书详情
          </Button>
          <Button variant="outline" onClick={() => navigate(`/tasks?certificateId=${id}`)}>
            <ListChecks />
            关联任务
          </Button>
        </div>
      }
    />
  );

  if (certQ.isLoading) {
    return (
      <div className="mx-auto max-w-3xl space-y-4 p-4 sm:p-6">
        <Skeleton className="h-8 w-56" />
        <Skeleton className="h-40 rounded-xl" />
      </div>
    );
  }

  if (certQ.isError || !certQ.data) {
    const notFound = certQ.error instanceof ApiError && certQ.error.code === "cert_not_found";
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        {header}
        {notFound ? (
          <EmptyState
            Icon={ShieldCheck}
            title="证书不存在"
            description="该证书可能已被删除。"
            action={<Button onClick={() => navigate("/certificates")}>返回证书列表</Button>}
          />
        ) : (
          <ErrorState error={certQ.error} onRetry={() => void certQ.refetch()} />
        )}
      </div>
    );
  }

  const cert = certQ.data;

  // 自签证书不经 ACME 验证。
  if (cert.issuanceMethod === "self_signed") {
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        {header}
        <EmptyState
          Icon={Landmark}
          title="自签证书不经 ACME 验证"
          description="该证书由自签根 CA 直接签发,不涉及域名验证挑战。"
          action={
            <Button variant="outline" onClick={() => navigate(`/certificates/${id}`)}>
              返回证书详情
            </Button>
          }
        />
      </div>
    );
  }

  const all = chalQ.data?.items ?? [];
  // 聚焦「本次」:取最近一次任务(挑战 createdAt 最大者所属 task)的各域名挑战。
  const latestTaskId = all.length
    ? all.reduce((a, b) => (a.createdAt >= b.createdAt ? a : b)).taskId
    : null;
  const challenges = latestTaskId ? all.filter((c) => c.taskId === latestTaskId) : [];
  challenges.sort((a, b) => (a.domainHostname ?? "").localeCompare(b.domainHostname ?? ""));

  return (
    <div className="mx-auto max-w-3xl space-y-4 p-4 sm:p-6">
      {header}

      {chalQ.isError ? (
        <ErrorState error={chalQ.error} onRetry={() => void chalQ.refetch()} />
      ) : chalQ.isLoading ? (
        <Skeleton className="h-40 rounded-xl" />
      ) : challenges.length === 0 ? (
        isInProgress(cert.status) ? (
          <OverallBanner challenges={[]} />
        ) : (
          <EmptyState
            Icon={BadgeCheck}
            title="暂无验证挑战记录"
            description="本证书当前没有可展示的验证挑战。发起签发 / 续签(公共 ACME)后,各域名的验证挑战将在此呈现。"
            action={
              <Button variant="outline" onClick={() => navigate(`/certificates/${id}`)}>
                返回证书详情
              </Button>
            }
          />
        )
      ) : (
        <>
          <OverallBanner challenges={challenges} />
          <div className="space-y-3">
            {challenges.map((c) => (
              <ChallengeCard key={c.id} challenge={c} />
            ))}
          </div>
          <p className="text-xs text-muted-foreground">
            多域名(SAN)各跑一个挑战、可分别处于不同状态;全部通过方可取证,任一失败 / 取消则整体失败。
          </p>
        </>
      )}
    </div>
  );
}
