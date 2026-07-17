/**
 * ACME 账户页(acme/accounts PRD)—— 多账户列表(状态 / 目标 CA / 环境 / 邮箱 / 只读默认标示)+ 配置注册 +
 * 编辑邮箱(仅已注册)+ 注册失败重试 + 移除(被证书引用给影响提示 + 二次确认)。默认账户设定归 settings(本页只读)。
 * 状态经 StatusBadge(§3.2);破坏性走 AlertDialog(H5);四态齐备(H3)。SSE acme_account_status_changed 实时刷新。
 */
import { useState } from "react";
import { useNavigate } from "react-router";
import {
  BadgeCheck,
  MoreHorizontal,
  Pencil,
  Plus,
  RotateCw,
  Settings as SettingsIcon,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import {
  useAcmeAccounts,
  useDeleteAcmeAccount,
  useRetryAcmeAccount,
  useUpdateAcmeAccountEmail,
} from "@/lib/queries";
import type { AcmeAccountSummary } from "@/bindings";
import { ApiError } from "@/lib/api";
import { PageHeader } from "@/components/shared/page-header";
import { EmptyState, ErrorState, TableSkeletonRows } from "@/components/shared/states";
import { StatusBadge } from "@/components/status-badge";
import { EnvironmentBadge } from "@/components/shared/category-badges";
import { Mono } from "@/components/shared/mono";
import { RegisterAccountDialog } from "@/components/acme/register-account-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { toast } from "@/components/ui/sonner";

const COLS = 5;

export function AcmeAccountsPage() {
  const navigate = useNavigate();
  const { data, isLoading, isError, error, refetch } = useAcmeAccounts();
  const retry = useRetryAcmeAccount();
  const del = useDeleteAcmeAccount();
  const updateEmail = useUpdateAcmeAccountEmail();

  const [registerOpen, setRegisterOpen] = useState(false);
  const [editing, setEditing] = useState<AcmeAccountSummary | null>(null);
  const [emailValue, setEmailValue] = useState("");
  const [emailError, setEmailError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<AcmeAccountSummary | null>(null);

  const items = data?.items ?? [];
  const showEmpty = !isLoading && !isError && items.length === 0;

  const onRetry = (a: AcmeAccountSummary) => {
    retry.mutate(a.id, {
      onSuccess: () => toast.success("已重新发起注册"),
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "重试失败"),
    });
  };

  const openEdit = (a: AcmeAccountSummary) => {
    setEditing(a);
    setEmailValue(a.contactEmail);
    setEmailError(null);
  };

  const submitEdit = () => {
    if (!editing) return;
    const email = emailValue.trim();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setEmailError("邮箱格式非法");
      return;
    }
    updateEmail.mutate(
      { id: editing.id, contactEmail: email },
      {
        onSuccess: () => {
          toast.success("联系邮箱已更新");
          setEditing(null);
        },
        onError: (e) => {
          if (e instanceof ApiError) setEmailError(e.message);
          else setEmailError("更新失败");
        },
      },
    );
  };

  const onRemove = () => {
    if (!removing) return;
    del.mutate(removing.id, {
      onSuccess: () => {
        toast.success("账户已移除");
        setRemoving(null);
      },
      onError: (e) => toast.error(e instanceof ApiError ? e.message : "移除失败"),
    });
  };

  return (
    <div className="p-4 sm:p-6">
      <PageHeader
        title="ACME 账户"
        description="管理 ACME 账户:配置注册、编辑联系邮箱、失败重试、移除。支持多账户并存(不同 CA / 生产·测试环境)。"
        actions={
          <Button onClick={() => setRegisterOpen(true)}>
            <Plus />
            配置账户
          </Button>
        }
      />

      <div className="overflow-hidden rounded-xl border border-border bg-card">
        {isError ? (
          <div className="p-4">
            <ErrorState error={error} onRetry={() => void refetch()} />
          </div>
        ) : showEmpty ? (
          <EmptyState
            Icon={BadgeCheck}
            title="尚无 ACME 账户"
            description="配置第一个 ACME 账户(选择目标 CA、填写邮箱并同意服务条款),注册后即可用于公共 ACME 签发。"
            action={
              <Button onClick={() => setRegisterOpen(true)}>
                <Plus />
                配置账户
              </Button>
            }
          />
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>目标 CA</TableHead>
                <TableHead>环境</TableHead>
                <TableHead>联系邮箱</TableHead>
                <TableHead>状态</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableSkeletonRows rows={3} cols={COLS} />
              ) : (
                items.map((a) => (
                  <TableRow key={a.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <span className="max-w-[160px] truncate font-medium">
                          {a.caLabel ?? "ACME CA"}
                        </span>
                        {a.isDefault && <Badge variant="outline">默认</Badge>}
                      </div>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <span className="cursor-help">
                            <Mono className="block max-w-[240px] truncate text-muted-foreground">
                              {a.directoryUrl}
                            </Mono>
                          </span>
                        </TooltipTrigger>
                        <TooltipContent className="max-w-md">
                          <Mono className="break-all">{a.directoryUrl}</Mono>
                        </TooltipContent>
                      </Tooltip>
                    </TableCell>
                    <TableCell>
                      <EnvironmentBadge environment={a.environment} />
                    </TableCell>
                    <TableCell>
                      <Mono className="block max-w-[200px] truncate">{a.contactEmail}</Mono>
                    </TableCell>
                    <TableCell>
                      <div className="space-y-1">
                        <StatusBadge status={a.status} />
                        {a.status === "registration_failed" && a.lastError && (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <p className="max-w-[200px] cursor-help truncate text-xs text-danger">
                                {a.lastError}
                              </p>
                            </TooltipTrigger>
                            <TooltipContent className="max-w-xs">
                              <p>{a.lastError}</p>
                            </TooltipContent>
                          </Tooltip>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            aria-label={`账户操作:${a.caLabel ?? a.directoryUrl}`}
                          >
                            <MoreHorizontal />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent>
                          {a.status === "registered" && (
                            <DropdownMenuItem onClick={() => openEdit(a)}>
                              <Pencil />
                              编辑联系邮箱
                            </DropdownMenuItem>
                          )}
                          {a.status === "registration_failed" && (
                            <DropdownMenuItem
                              onClick={() => onRetry(a)}
                              disabled={retry.isPending}
                            >
                              <RotateCw />
                              重新注册
                            </DropdownMenuItem>
                          )}
                          {a.status === "registering" && (
                            <DropdownMenuItem disabled>注册中,暂无可用操作</DropdownMenuItem>
                          )}
                          {(a.status === "registered" ||
                            a.status === "registration_failed") && (
                            <>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem variant="danger" onClick={() => setRemoving(a)}>
                                <Trash2 />
                                移除账户
                              </DropdownMenuItem>
                            </>
                          )}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        )}
      </div>

      {items.length > 0 && (
        <p className="mt-3 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
          <span>共 {data?.total ?? items.length} 个账户 · 默认签发账户在设置页指定,本页仅只读呈现。</span>
          <Button
            variant="link"
            size="sm"
            className="h-auto gap-1 p-0 text-xs"
            onClick={() => navigate("/settings")}
          >
            <SettingsIcon className="size-3" />
            前往设置
          </Button>
        </p>
      )}

      <RegisterAccountDialog open={registerOpen} onOpenChange={setRegisterOpen} />

      {/* 编辑联系邮箱(仅已注册) */}
      <Dialog
        open={!!editing}
        onOpenChange={(o) => {
          if (!o && !updateEmail.isPending) setEditing(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>编辑联系邮箱</DialogTitle>
            <DialogDescription>
              更新账户联系邮箱;业务上仍为同一账户,不改变账户状态。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-1.5">
            <Label htmlFor="edit-email">联系邮箱</Label>
            <Input
              id="edit-email"
              type="email"
              className="font-mono text-[13px]"
              value={emailValue}
              aria-invalid={!!emailError}
              onChange={(e) => {
                setEmailValue(e.target.value);
                setEmailError(null);
              }}
            />
            {emailError && <p className="text-xs text-danger">{emailError}</p>}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setEditing(null)}
              disabled={updateEmail.isPending}
            >
              取消
            </Button>
            <Button onClick={submitEdit} disabled={updateEmail.isPending}>
              {updateEmail.isPending && <RotateCw className="animate-spin" />}
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 移除账户(影响提示 + 二次确认,H5) */}
      <AlertDialog
        open={!!removing}
        onOpenChange={(o) => {
          if (!o && !del.isPending) setRemoving(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>移除该 ACME 账户?</AlertDialogTitle>
            <AlertDialogDescription>
              移除后账户退出状态机、清除本地账户密钥材料,不可撤销。
            </AlertDialogDescription>
          </AlertDialogHeader>
          {removing && removing.certificateCount > 0 && (
            <Alert variant="warning">
              <TriangleAlert />
              <AlertTitle>有 {removing.certificateCount} 张证书正引用该账户</AlertTitle>
              <AlertDescription>
                这些证书的签发账户将被置空,后续续签需改用其他账户;若该账户为默认账户,默认指向也将清空。
              </AlertDescription>
            </Alert>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel disabled={del.isPending}>取消</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={del.isPending}
              onClick={(e) => {
                e.preventDefault();
                onRemove();
              }}
            >
              移除
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
