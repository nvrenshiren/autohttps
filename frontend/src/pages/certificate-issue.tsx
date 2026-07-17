/**
 * 发起签发(certificates/issue PRD)—— 选域名(SAN,≤1 通配符,F1)+ 选签发方式(ACME / 自签,F2)+
 * 触发首次签发(F4)→ POST /certificates → 跳详情看进度。自签路径全链路已通;ACME 验证向导归 acme
 * 模块(F3,本页仅衔接入口)。校验错误行内,提交结果 toast。
 */
import { useMemo, useState } from "react";
import { useNavigate } from "react-router";
import {
  BadgeCheck,
  Check,
  Globe,
  KeyRound,
  Landmark,
  Loader2,
  Search,
  ShieldCheck,
} from "lucide-react";
import {
  useAcmeAccounts,
  useCreateCertificate,
  useDomains,
  useRootCas,
} from "@/lib/queries";
import type { IssuanceMethod } from "@/bindings";
import { ApiError } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/shared/page-header";
import { ErrorState } from "@/components/shared/states";
import { WildcardBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "@/components/ui/sonner";

export function CertificateIssuePage() {
  const navigate = useNavigate();
  const create = useCreateCertificate();

  const domainsQ = useDomains({ page: 1, pageSize: 200, sort: "hostname", order: "asc" });
  const rootCasQ = useRootCas({ page: 1, pageSize: 100, status: "active" });
  const accountsQ = useAcmeAccounts("registered");

  const [method, setMethod] = useState<IssuanceMethod>("self_signed");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [rootCaId, setRootCaId] = useState<string>("");
  const [acmeAccountId, setAcmeAccountId] = useState<string>("");
  const [search, setSearch] = useState("");
  const [formError, setFormError] = useState<string | null>(null);

  const domains = domainsQ.data?.items ?? [];
  const rootCas = rootCasQ.data?.items ?? [];
  const accounts = accountsQ.data?.items ?? [];

  const filteredDomains = useMemo(
    () => domains.filter((d) => d.hostname.toLowerCase().includes(search.trim().toLowerCase())),
    [domains, search],
  );

  const selectedDomains = domains.filter((d) => selected.has(d.id));
  const wildcardCount = selectedDomains.filter((d) => d.isWildcard).length;
  const tooManyWildcards = wildcardCount > 1;

  const toggle = (id: string) => {
    setFormError(null);
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const canSubmit =
    selected.size > 0 &&
    !tooManyWildcards &&
    (method === "self_signed" ? rootCaId !== "" : acmeAccountId !== "") &&
    !create.isPending;

  const onSubmit = (ev: React.FormEvent) => {
    ev.preventDefault();
    setFormError(null);
    if (selected.size === 0) {
      setFormError("请至少选择一个域名组成签发范围(SAN)。");
      return;
    }
    if (tooManyWildcards) {
      setFormError("一次签发至多包含一个通配符域名。");
      return;
    }
    if (method === "self_signed" && !rootCaId) {
      setFormError("请选择用于签发的根 CA。");
      return;
    }
    if (method === "acme" && !acmeAccountId) {
      setFormError("请选择用于签发的 ACME 账户。");
      return;
    }

    create.mutate(
      {
        issuanceMethod: method,
        domainIds: [...selected],
        rootCaId: method === "self_signed" ? rootCaId : undefined,
        acmeAccountId: method === "acme" ? acmeAccountId : undefined,
      },
      {
        onSuccess: (cert) => {
          toast.success("已发起签发");
          // ACME:引导到验证方式向导完成域名验证(DNS-01 需手动加 TXT);自签直接看详情进度。
          navigate(
            method === "acme"
              ? `/certificates/${cert.id}/challenges`
              : `/certificates/${cert.id}`,
          );
        },
        onError: (e) => {
          setFormError(e instanceof ApiError ? e.message : "发起签发失败");
        },
      },
    );
  };

  return (
    <div className="mx-auto max-w-3xl p-4 sm:p-6">
      <PageHeader
        title="发起签发"
        crumbs={[{ label: "证书", to: "/certificates" }, { label: "发起签发" }]}
      />

      <form onSubmit={onSubmit} className="space-y-5">
        {/* 选择域名 */}
        <Card className="gap-0 overflow-hidden py-0">
          <CardHeader className="flex-row items-center justify-between border-b border-border py-4">
            <CardTitle className="text-sm">
              选择域名(SAN)
              {selected.size > 0 && (
                <span className="ml-2 font-normal text-muted-foreground">
                  已选 {selected.size} 个
                </span>
              )}
            </CardTitle>
            <div className="relative w-48">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="搜索域名…"
                className="h-8 pl-8"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
          </CardHeader>
          <CardContent className="p-3">
            {domainsQ.isLoading ? (
              <div className="space-y-2">
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-full" />
                <Skeleton className="h-9 w-full" />
              </div>
            ) : domainsQ.isError ? (
              <ErrorState error={domainsQ.error} onRetry={() => void domainsQ.refetch()} />
            ) : domains.length === 0 ? (
              <div className="flex flex-col items-center gap-2 py-8 text-center">
                <Globe className="size-7 text-muted-foreground" />
                <div className="text-sm font-medium">尚无域名</div>
                <div className="max-w-sm text-xs text-muted-foreground">
                  证书必须关联已存在的域名。请先到域名管理新增域名。
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="mt-1"
                  onClick={() => navigate("/domains")}
                >
                  前往域名管理
                </Button>
              </div>
            ) : filteredDomains.length === 0 ? (
              <div className="py-8 text-center text-xs text-muted-foreground">
                没有匹配「{search}」的域名。
              </div>
            ) : (
              <div className="max-h-56 space-y-1 overflow-auto">
                {filteredDomains.map((d) => {
                  const isSel = selected.has(d.id);
                  return (
                    <button
                      key={d.id}
                      type="button"
                      onClick={() => toggle(d.id)}
                      aria-pressed={isSel}
                      className={cn(
                        "flex w-full items-center gap-2 rounded-md border px-2.5 py-1.5 text-left transition-colors",
                        isSel
                          ? "border-primary bg-accent"
                          : "border-border hover:bg-accent/50",
                      )}
                    >
                      <span
                        className={cn(
                          "inline-flex size-4 shrink-0 items-center justify-center rounded border",
                          isSel ? "border-primary bg-primary text-primary-foreground" : "border-input",
                        )}
                      >
                        {isSel && <Check className="size-3" />}
                      </span>
                      <Mono className="min-w-0 flex-1 truncate">{d.hostname}</Mono>
                      {d.isWildcard && <WildcardBadge />}
                    </button>
                  );
                })}
              </div>
            )}
            {tooManyWildcards && (
              <p className="mt-2 text-xs text-danger">一次签发至多包含一个通配符域名。</p>
            )}
          </CardContent>
        </Card>

        {/* 签发方式 */}
        <Card>
          <CardHeader>
            <CardTitle className="text-sm">签发方式</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 pb-6">
            <div className="inline-flex rounded-lg border border-border bg-muted/40 p-1">
              {(
                [
                  { m: "self_signed" as const, label: "自签根 CA", Icon: Landmark },
                  { m: "acme" as const, label: "公共 ACME", Icon: KeyRound },
                ]
              ).map(({ m, label, Icon }) => (
                <button
                  key={m}
                  type="button"
                  onClick={() => {
                    setMethod(m);
                    setFormError(null);
                  }}
                  className={cn(
                    "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                    method === m
                      ? "bg-background text-foreground shadow-xs"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  <Icon className="size-4" />
                  {label}
                </button>
              ))}
            </div>

            {method === "self_signed" ? (
              <div className="space-y-1.5">
                <Label>根 CA</Label>
                {rootCasQ.isLoading ? (
                  <Skeleton className="h-9 w-full max-w-sm" />
                ) : rootCas.length === 0 ? (
                  <Alert variant="warning">
                    <ShieldCheck />
                    <AlertTitle>尚无有效的根 CA</AlertTitle>
                    <AlertDescription>
                      <p>自签签发需要一个有效(active)的根 CA。请先创建或导入根 CA。</p>
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        className="mt-1 w-fit"
                        onClick={() => navigate("/local-ca/new")}
                      >
                        <Landmark />
                        新增根 CA
                      </Button>
                    </AlertDescription>
                  </Alert>
                ) : (
                  <Select value={rootCaId} onValueChange={setRootCaId}>
                    <SelectTrigger className="w-full max-w-sm">
                      <SelectValue placeholder="选择用于签发的根 CA" />
                    </SelectTrigger>
                    <SelectContent>
                      {rootCas.map((ca) => (
                        <SelectItem key={ca.id} value={ca.id}>
                          {ca.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
                <p className="text-xs text-muted-foreground">
                  由根 CA 直接签发内网证书,不经 ACME 域名验证;签发为本地操作,证书很快转「有效」。
                </p>
              </div>
            ) : (
              <div className="space-y-3">
                <div className="space-y-1.5">
                  <Label>ACME 账户</Label>
                  {accountsQ.isLoading ? (
                    <Skeleton className="h-9 w-full max-w-sm" />
                  ) : accounts.length === 0 ? (
                    <Alert variant="warning">
                      <BadgeCheck />
                      <AlertTitle>尚无已注册的 ACME 账户</AlertTitle>
                      <AlertDescription>
                        <p>公共 ACME 签发需要一个已注册的账户。账户配置与注册在 ACME 模块。</p>
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          className="mt-1 w-fit"
                          onClick={() => navigate("/acme")}
                        >
                          <BadgeCheck />
                          前往 ACME
                        </Button>
                      </AlertDescription>
                    </Alert>
                  ) : (
                    <Select value={acmeAccountId} onValueChange={setAcmeAccountId}>
                      <SelectTrigger className="w-full max-w-sm">
                        <SelectValue placeholder="选择用于签发的 ACME 账户" />
                      </SelectTrigger>
                      <SelectContent>
                        {accounts.map((a) => (
                          <SelectItem key={a.id} value={a.id}>
                            {a.caLabel ?? a.directoryUrl}
                            {a.environment ? ` · ${a.environment}` : ""}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                </div>
                <Alert>
                  <KeyRound />
                  <AlertTitle>验证方式在 ACME 模块衔接</AlertTitle>
                  <AlertDescription>
                    发起后经 ACME 验证域名控制权(HTTP-01 / DNS-01);通配符域名限 DNS-01。验证向导明细在 ACME 模块,本页仅衔接入口。
                  </AlertDescription>
                </Alert>
              </div>
            )}
          </CardContent>
        </Card>

        {formError && (
          <Alert variant="destructive">
            <AlertDescription>{formError}</AlertDescription>
          </Alert>
        )}

        <div className="flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="outline"
            disabled={create.isPending}
            onClick={() => navigate("/certificates")}
          >
            取消
          </Button>
          <Button type="submit" disabled={!canSubmit}>
            {create.isPending && <Loader2 className="animate-spin" />}
            发起签发
          </Button>
        </div>
      </form>
    </div>
  );
}
