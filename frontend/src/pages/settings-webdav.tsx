/**
 * WebDAV 备份同步卡片(设置页分区)—— 手动快照备份/恢复(sync PRD MVP)。
 *
 * - 配置表单(URL/用户名/口令,口令只写不读,`passwordSet` 显示已存状态)+ 测试连接;
 * - 立即备份:口令弹窗(≥10 位,口令即私钥最后防线,醒目提示);
 * - 远端备份列表 + 恢复(口令 + 明确「重启后生效」确认);
 * - 未配置时仅渲染配置表单,备份/列表区块折叠。
 */
import { useEffect, useState } from "react";
import {
  CloudUpload,
  Loader2,
  RefreshCw,
  RotateCcw,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import {
  useBackupNow,
  useDeleteSyncConfig,
  useRemoteBackups,
  useRestoreBackup,
  useSaveSyncConfig,
  useSyncConfig,
  useTestSyncConnection,
} from "@/lib/queries";
import { ApiError } from "@/lib/api";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
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
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "@/components/ui/sonner";

const MIN_PASSPHRASE_LEN = 10;

function errMsg(e: unknown, fallback: string): string {
  return e instanceof ApiError ? e.message : fallback;
}

function fmtSize(size: number | null): string {
  if (size === null) return "-";
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / 1024 / 1024).toFixed(2)} MB`;
}

export function WebdavSyncCard() {
  const config = useSyncConfig();
  const save = useSaveSyncConfig();
  const del = useDeleteSyncConfig();
  const test = useTestSyncConnection();
  const backup = useBackupNow();
  const restore = useRestoreBackup();
  const configured = config.data?.configured ?? false;
  const backups = useRemoteBackups(configured);

  const [serverUrl, setServerUrl] = useState("");
  const [remoteDir, setRemoteDir] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  useEffect(() => {
    if (config.data) {
      setServerUrl(config.data.serverUrl ?? "");
      setRemoteDir(config.data.remoteDir ?? "");
      setUsername(config.data.username ?? "");
      // 口令不回显:已存则留空,占位文案提示
      setPassword("");
    }
  }, [config.data]);

  // 备份口令弹窗 / 恢复确认
  const [backupOpen, setBackupOpen] = useState(false);
  const [passphrase, setPassphrase] = useState("");
  const [restoring, setRestoring] = useState<string | null>(null);
  const [restorePassphrase, setRestorePassphrase] = useState("");
  const [confirmRemove, setConfirmRemove] = useState(false);

  if (config.isLoading) {
    return <Skeleton className="h-48 rounded-xl" />;
  }
  if (config.isError) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>WebDAV 备份同步</CardTitle>
        </CardHeader>
        <CardContent>
          <Alert variant="destructive">
            <TriangleAlert />
            <AlertDescription>{errMsg(config.error, "加载同步配置失败")}</AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    );
  }

  const onSave = () => {
    save.mutate(
      {
        serverUrl: serverUrl.trim(),
        remoteDir: remoteDir.trim() || undefined,
        username: username.trim(),
        // 口令留空且已存过 → 不传(保留);未存过且留空 → 也不传,由测试连接暴露缺口令
        ...(password !== "" ? { password } : {}),
      },
      {
        onSuccess: () => toast.success("WebDAV 配置已保存"),
        onError: (e) => toast.error(errMsg(e, "保存失败")),
      },
    );
  };

  const onTest = () => {
    test.mutate(undefined, {
      onSuccess: () => toast.success("连接成功,远端目录可用"),
      onError: (e) => toast.error(errMsg(e, "连接失败")),
    });
  };

  const onBackup = () => {
    if (passphrase.length < MIN_PASSPHRASE_LEN) return;
    backup.mutate(passphrase, {
      onSuccess: (item) => {
        toast.success(`备份已上传:${item.name}`);
        setBackupOpen(false);
        setPassphrase("");
        void backups.refetch();
      },
      onError: (e) => toast.error(errMsg(e, "备份失败")),
    });
  };

  const onRestore = () => {
    if (!restoring || restorePassphrase.length < 1) return;
    restore.mutate(
      { remoteName: restoring, passphrase: restorePassphrase },
      {
        onSuccess: (outcome) => {
          setRestoring(null);
          setRestorePassphrase("");
          toast.success("恢复完成 —— 请重启应用使数据生效", {
            description: `已从 ${outcome.backupCreatedAt} 的备份恢复(${outcome.secretsRestored} 份密钥材料)。当前现场已归档,可回滚。`,
            duration: 10_000,
          });
        },
        onError: (e) => toast.error(errMsg(e, "恢复失败")),
      },
    );
  };

  const lastResult = config.data?.lastBackupResult;

  return (
    <Card>
      <CardHeader>
        <CardTitle>WebDAV 备份同步</CardTitle>
        <CardDescription>
          手动快照备份到 WebDAV:整库 + 全部密钥材料打包后用你的口令加密再上传。
          口令是唯一防线 —— 服务端/我们都无法替你解密。
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* 配置表单 */}
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-1.5">
            <Label htmlFor="webdavServer">服务器地址</Label>
            <Input
              id="webdavServer"
              className="font-mono text-[13px]"
              placeholder="https://dav.example.com/dav"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="webdavDir">远程目录</Label>
            <Input
              id="webdavDir"
              className="font-mono text-[13px]"
              placeholder="autohttps(默认)"
              value={remoteDir}
              onChange={(e) => setRemoteDir(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="webdavUser">用户名</Label>
            <Input
              id="webdavUser"
              autoComplete="off"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="webdavPass">密码</Label>
            <Input
              id="webdavPass"
              type="password"
              autoComplete="new-password"
              placeholder={config.data?.passwordSet ? "已保存(留空则不修改)" : "未设置"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>
        </div>
        <p className="text-xs text-muted-foreground">
          备份将上传到独立远程目录(默认 <span className="font-mono">autohttps/</span>),
          与同一 WebDAV 上其他项目的文件隔离,不会散在根目录。
        </p>
        {configured && config.data?.baseUrl && (
          <div className="rounded-md border border-border bg-muted/50 px-3 py-2 text-xs">
            <span className="text-muted-foreground">备份位置:</span>{" "}
            <span className="font-mono">{config.data.baseUrl}</span>
            <span className="ml-2 text-muted-foreground">
              · 用户 {config.data.username} · 口令{config.data.passwordSet ? "已保存" : "未设置"}
            </span>
          </div>
        )}
        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            onClick={onSave}
            disabled={save.isPending || !serverUrl.trim() || !username.trim()}
          >
            {save.isPending && <Loader2 className="animate-spin" />}
            保存配置
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={onTest}
            disabled={test.isPending || !configured}
          >
            {test.isPending && <Loader2 className="animate-spin" />}
            测试连接
          </Button>
          {configured && (
            <Button
              size="sm"
              variant="ghost"
              className="text-destructive hover:text-destructive"
              onClick={() => setConfirmRemove(true)}
            >
              <Trash2 />
              清除配置
            </Button>
          )}
          {lastResult && (
            <span className="text-xs text-muted-foreground">
              上次备份:
              {lastResult === "success"
                ? `成功 · ${config.data?.lastBackupAt ?? "-"}`
                : `失败 · ${config.data?.lastBackupError ?? "未知原因"}`}
            </span>
          )}
        </div>

        {configured && (
          <>
            <Separator />
            {/* 备份动作 + 远端列表 */}
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="space-y-0.5">
                <Label>远端备份</Label>
                <p className="text-xs text-muted-foreground">
                  备份包为 age 口令加密的完整快照(库 + 密钥材料);远端文件只增不删。
                </p>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => void backups.refetch()}
                  disabled={backups.isFetching}
                >
                  {backups.isFetching ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <RefreshCw />
                  )}
                  刷新
                </Button>
                <Button size="sm" onClick={() => setBackupOpen(true)}>
                  <CloudUpload />
                  立即备份
                </Button>
              </div>
            </div>

            {backups.isLoading ? (
              <Skeleton className="h-24 rounded-lg" />
            ) : backups.isError ? (
              <Alert variant="destructive">
                <TriangleAlert />
                <AlertDescription>
                  {errMsg(backups.error, "获取远端备份列表失败")}
                </AlertDescription>
              </Alert>
            ) : (backups.data?.length ?? 0) === 0 ? (
              <p className="rounded-lg border border-dashed border-border px-4 py-6 text-center text-sm text-muted-foreground">
                远端还没有备份。点击「立即备份」创建第一份快照。
              </p>
            ) : (
              <div className="divide-y divide-border rounded-lg border border-border">
                {backups.data!.map((b) => (
                  <div
                    key={b.name}
                    className="flex items-center justify-between gap-3 px-3 py-2 text-sm"
                  >
                    <div className="min-w-0">
                      <p className="truncate font-mono text-[13px]">{b.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {fmtSize(b.size)}
                        {b.modified ? ` · ${b.modified}` : ""}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => {
                        setRestoring(b.name);
                        setRestorePassphrase("");
                      }}
                    >
                      <RotateCcw />
                      恢复
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </CardContent>

      {/* 备份口令弹窗 */}
      <Dialog
        open={backupOpen}
        onOpenChange={(o) => {
          if (!o && !backup.isPending) {
            setBackupOpen(false);
            setPassphrase("");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>设置备份加密口令</DialogTitle>
            <DialogDescription>
              备份包含全部私钥材料(主密钥在内)。该口令是私钥安全的最后防线,
              请使用 ≥{MIN_PASSPHRASE_LEN} 位的强口令并自行妥善保管 —— 遗失无法恢复备份。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-1.5">
            <Label htmlFor="backupPassphrase">加密口令</Label>
            <Input
              id="backupPassphrase"
              type="password"
              autoComplete="new-password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
            />
            {passphrase !== "" && passphrase.length < MIN_PASSPHRASE_LEN && (
              <p className="text-xs text-destructive">
                口令至少 {MIN_PASSPHRASE_LEN} 位(当前 {passphrase.length} 位)
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setBackupOpen(false)} disabled={backup.isPending}>
              取消
            </Button>
            <Button
              onClick={onBackup}
              disabled={backup.isPending || passphrase.length < MIN_PASSPHRASE_LEN}
            >
              {backup.isPending && <Loader2 className="animate-spin" />}
              加密并上传
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 恢复确认弹窗 */}
      <Dialog
        open={!!restoring}
        onOpenChange={(o) => {
          if (!o && !restore.isPending) {
            setRestoring(null);
            setRestorePassphrase("");
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>从备份恢复?</DialogTitle>
            <DialogDescription>
              将用 <span className="font-mono text-[12px]">{restoring}</span> 覆盖当前全部数据。
              当前现场会先归档(可回滚),恢复完成后需重启应用生效。
            </DialogDescription>
          </DialogHeader>
          <Alert variant="warning">
            <TriangleAlert />
            <AlertTitle>覆盖式恢复</AlertTitle>
            <AlertDescription>
              恢复后当前库与密钥材料被备份内容替换;恢复期间请勿操作其他页面。
            </AlertDescription>
          </Alert>
          <div className="space-y-1.5">
            <Label htmlFor="restorePassphrase">备份口令</Label>
            <Input
              id="restorePassphrase"
              type="password"
              autoComplete="off"
              placeholder="创建该备份时设置的加密口令"
              value={restorePassphrase}
              onChange={(e) => setRestorePassphrase(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRestoring(null)} disabled={restore.isPending}>
              取消
            </Button>
            <Button
              variant="destructive"
              onClick={onRestore}
              disabled={restore.isPending || restorePassphrase.length < 1}
            >
              {restore.isPending && <Loader2 className="animate-spin" />}
              覆盖并恢复
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 清除配置确认 */}
      <AlertDialog open={confirmRemove} onOpenChange={setConfirmRemove}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>清除 WebDAV 配置?</AlertDialogTitle>
            <AlertDialogDescription>
              将删除本地保存的地址、用户名与口令密文;远端已有备份不受影响。
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={del.isPending}>取消</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={del.isPending}
              onClick={(e) => {
                e.preventDefault();
                del.mutate(undefined, {
                  onSuccess: () => {
                    toast.success("已清除 WebDAV 配置");
                    setConfirmRemove(false);
                  },
                  onError: (err) => toast.error(errMsg(err, "清除失败")),
                });
              }}
            >
              清除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </Card>
  );
}
