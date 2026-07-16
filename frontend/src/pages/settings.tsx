/**
 * 设置(settings PRD)—— 单页分区(Card,非 Tabs;H12)。续签策略 + 运行形态项(仅桌面/仅服务器,
 * 依 runMode 显隐,H10)+ 数据存储(只读)。监听地址设为对外可达 → 风险 Alert(H6)。
 */
import { useEffect, useState } from "react";
import { Info, Loader2, TriangleAlert } from "lucide-react";
import { useAppInfo, useSettings, useUpdateSettings } from "@/lib/queries";
import type { UpdateSettingsRequest } from "@/bindings";
import { PageHeader } from "@/components/shared/page-header";
import { ErrorState } from "@/components/shared/states";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Mono } from "@/components/shared/mono";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "@/components/ui/sonner";
import { ApiError } from "@/lib/api";

interface FormState {
  renewalAdvanceDays: number;
  autoRenewEnabled: boolean;
  autostartEnabled: boolean;
  listenAddress: string;
  listenPort: number | "";
}

function isPublicAddress(addr: string): boolean {
  const a = addr.trim().toLowerCase();
  if (!a) return false;
  return !(a === "127.0.0.1" || a === "localhost" || a === "::1");
}

export function SettingsPage() {
  const { data, isLoading, isError, error, refetch } = useSettings();
  const appInfo = useAppInfo();
  const isDesktop = appInfo.data?.runMode === "desktop";
  const update = useUpdateSettings();

  const [form, setForm] = useState<FormState>({
    renewalAdvanceDays: 30,
    autoRenewEnabled: true,
    autostartEnabled: false,
    listenAddress: "",
    listenPort: "",
  });

  useEffect(() => {
    if (data) {
      setForm({
        renewalAdvanceDays: data.renewalAdvanceDays,
        autoRenewEnabled: data.autoRenewEnabled,
        autostartEnabled: data.autostartEnabled ?? false,
        listenAddress: data.listenAddress ?? "",
        listenPort: data.listenPort ?? "",
      });
    }
  }, [data]);

  const onSave = () => {
    const body: UpdateSettingsRequest = {
      renewalAdvanceDays: form.renewalAdvanceDays,
      autoRenewEnabled: form.autoRenewEnabled,
    };
    if (isDesktop) {
      body.autostartEnabled = form.autostartEnabled;
    } else {
      body.listenAddress = form.listenAddress;
      if (form.listenPort !== "") body.listenPort = form.listenPort;
    }
    update.mutate(body, {
      onSuccess: () => toast.success("设置已保存"),
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "保存失败"),
    });
  };

  if (isLoading) {
    return (
      <div className="mx-auto max-w-3xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-8 w-40" />
        <Skeleton className="h-48 rounded-xl" />
        <Skeleton className="h-40 rounded-xl" />
      </div>
    );
  }
  if (isError || !data) {
    return (
      <div className="mx-auto max-w-3xl p-4 sm:p-6">
        <PageHeader title="设置" />
        <ErrorState error={error} onRetry={() => void refetch()} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-4 sm:p-6">
      <PageHeader
        title="设置"
        description="续签策略与运行形态配置。数据存储路径与运行形态为只读。"
        actions={
          <Button onClick={onSave} disabled={update.isPending}>
            {update.isPending && <Loader2 className="animate-spin" />}
            保存
          </Button>
        }
      />

      {/* 续签策略 */}
      <Card>
        <CardHeader>
          <CardTitle>续签策略</CardTitle>
          <CardDescription>控制"即将到期"判定与自动续签行为。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="renewalAdvanceDays">续签提前天数</Label>
            <Input
              id="renewalAdvanceDays"
              type="number"
              min={1}
              className="max-w-40"
              value={form.renewalAdvanceDays}
              onChange={(e) =>
                setForm((f) => ({ ...f, renewalAdvanceDays: Number(e.target.value) || 1 }))
              }
            />
            <p className="text-xs text-muted-foreground">
              证书到期前该天数内进入「即将到期」并触发自动续签(≥1)。
            </p>
          </div>
          <Separator />
          <div className="flex items-center justify-between gap-4">
            <div className="space-y-0.5">
              <Label htmlFor="autoRenew">自动续签</Label>
              <p className="text-xs text-muted-foreground">
                开启后「即将到期」证书自动续签,续签失败依扫描周期再尝试。
              </p>
            </div>
            <Switch
              id="autoRenew"
              checked={form.autoRenewEnabled}
              onCheckedChange={(v) => setForm((f) => ({ ...f, autoRenewEnabled: v }))}
            />
          </div>
        </CardContent>
      </Card>

      {/* 默认 ACME 账户(只读展示;账户管理在 ACME 页) */}
      <Card>
        <CardHeader>
          <CardTitle>默认 ACME 账户</CardTitle>
          <CardDescription>签发时的默认账户;账户的配置与注册在「ACME」页。</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="text-sm">
            当前默认账户:
            {data.defaultAcmeAccountId ? (
              <Mono className="ml-1">{data.defaultAcmeAccountId}</Mono>
            ) : (
              <span className="ml-1 text-muted-foreground">未设置</span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* 运行形态项:仅桌面 vs 仅服务器(H10 依 runMode 显隐) */}
      <Card>
        <CardHeader>
          <CardTitle>{isDesktop ? "桌面" : "服务器"}选项</CardTitle>
          <CardDescription>
            {isDesktop
              ? "桌面形态专属配置(仅桌面)。"
              : "守护进程监听配置(仅服务器)。"}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isDesktop ? (
            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="autostart">开机自启</Label>
                <p className="text-xs text-muted-foreground">随系统启动自动运行。</p>
              </div>
              <Switch
                id="autostart"
                checked={form.autostartEnabled}
                onCheckedChange={(v) => setForm((f) => ({ ...f, autostartEnabled: v }))}
              />
            </div>
          ) : (
            <>
              <div className="grid gap-4 sm:grid-cols-[1fr_140px]">
                <div className="space-y-1.5">
                  <Label htmlFor="listenAddress">监听地址</Label>
                  <Input
                    id="listenAddress"
                    className="font-mono text-[13px]"
                    value={form.listenAddress}
                    onChange={(e) => setForm((f) => ({ ...f, listenAddress: e.target.value }))}
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="listenPort">端口</Label>
                  <Input
                    id="listenPort"
                    type="number"
                    className="font-mono text-[13px]"
                    value={form.listenPort}
                    onChange={(e) =>
                      setForm((f) => ({
                        ...f,
                        listenPort: e.target.value === "" ? "" : Number(e.target.value),
                      }))
                    }
                  />
                </div>
              </div>
              {isPublicAddress(form.listenAddress) && (
                <Alert variant="destructive">
                  <TriangleAlert />
                  <AlertTitle>公网暴露风险</AlertTitle>
                  <AlertDescription>
                    监听地址非本机回环 —— Web UI 将可被网络内其他主机访问。MVP 无应用层鉴权,
                    安全由部署边界(防火墙 / 可信内网)保障,请确认该地址处于可信网络。
                  </AlertDescription>
                </Alert>
              )}
            </>
          )}
        </CardContent>
      </Card>

      {/* 数据存储(只读) */}
      <Card>
        <CardHeader>
          <CardTitle>数据存储</CardTitle>
          <CardDescription>存储路径与运行形态为只读(运行期不可改、无迁移)。</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1.5">
            <Label>数据存储路径</Label>
            <div className="rounded-md border border-border bg-muted/50 px-3 py-2">
              <Mono>{data.dataStoragePath}</Mono>
            </div>
          </div>
          <Alert>
            <Info />
            <AlertDescription>
              运行形态:{isDesktop ? "桌面" : "服务器"}(由运行载体探测,不可切换)。
              敏感私钥 / 账户密钥密文落该路径下,库内仅存引用键。
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    </div>
  );
}
