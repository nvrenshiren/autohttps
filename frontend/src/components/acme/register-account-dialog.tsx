/**
 * 配置并注册 ACME 账户(acme/accounts F2,AT1)—— react-hook-form + zod。
 * 选目标 CA(预设 / 自定义 directory URL)+ 联系邮箱 + 勾选同意服务条款(未勾选禁用注册,验收4)。
 * 提交 → POST /acme/accounts(202,registering);终态经 SSE 回推。校验错误行内,服务端错误 toast / 字段错误。
 */
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Loader2 } from "lucide-react";
import { useRegisterAcmeAccount } from "@/lib/queries";
import { ApiError } from "@/lib/api";
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
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "@/components/ui/sonner";

/** CA 预设(directoryUrl 唯一标定 CA + 环境;caLabel 仅展示名)。custom 时由用户填。 */
const PRESETS: Record<string, { directoryUrl: string; caLabel: string }> = {
  le_prod: {
    directoryUrl: "https://acme-v02.api.letsencrypt.org/directory",
    caLabel: "Let's Encrypt",
  },
  le_staging: {
    directoryUrl: "https://acme-staging-v02.api.letsencrypt.org/directory",
    caLabel: "Let's Encrypt (Staging)",
  },
};

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

const schema = z
  .object({
    preset: z.enum(["le_prod", "le_staging", "custom"]),
    directoryUrl: z.string().optional(),
    caLabel: z.string().optional(),
    contactEmail: z.string().min(1, "请输入联系邮箱").regex(EMAIL_RE, "邮箱格式非法"),
    tosAgreed: z.boolean(),
  })
  .refine((d) => d.tosAgreed, {
    message: "须勾选同意该 CA 的服务条款后方可注册",
    path: ["tosAgreed"],
  })
  .refine(
    (d) => d.preset !== "custom" || /^https?:\/\/.+/.test((d.directoryUrl ?? "").trim()),
    { message: "请输入合法的 ACME 目录 URL(https://…/directory)", path: ["directoryUrl"] },
  );

type FormValues = z.infer<typeof schema>;

export function RegisterAccountDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const register = useRegisterAcmeAccount();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      preset: "le_staging",
      directoryUrl: "",
      caLabel: "",
      contactEmail: "",
      tosAgreed: false,
    },
  });
  const {
    register: reg,
    control,
    handleSubmit,
    reset,
    watch,
    setError,
    formState: { errors },
  } = form;

  const preset = watch("preset");
  const isCustom = preset === "custom";

  const close = (o: boolean) => {
    if (!register.isPending) {
      if (!o) reset();
      onOpenChange(o);
    }
  };

  const onSubmit = handleSubmit((values) => {
    const resolved =
      values.preset === "custom"
        ? {
            directoryUrl: (values.directoryUrl ?? "").trim(),
            caLabel: values.caLabel?.trim() || undefined,
          }
        : { directoryUrl: PRESETS[values.preset].directoryUrl, caLabel: PRESETS[values.preset].caLabel };

    register.mutate(
      {
        directoryUrl: resolved.directoryUrl,
        caLabel: resolved.caLabel,
        contactEmail: values.contactEmail.trim(),
        tosAgreed: true,
      },
      {
        onSuccess: () => {
          toast.success("已发起账户注册,正在向 CA 注册…");
          reset();
          onOpenChange(false);
        },
        onError: (e) => {
          if (e instanceof ApiError) {
            if (e.code === "tos_not_agreed") setError("tosAgreed", { message: e.message });
            else if (e.code === "invalid_directory_url")
              setError("directoryUrl", { message: e.message });
            else if (e.code === "validation_failed")
              setError("contactEmail", { message: e.message });
            else toast.error(e.message);
          } else {
            toast.error("注册失败");
          }
        },
      },
    );
  });

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>配置并注册 ACME 账户</DialogTitle>
          <DialogDescription>
            选择目标 CA、填写联系邮箱并同意服务条款后发起注册;账户先进入「注册中」,由 CA 确认后转「已注册」。
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={onSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label>
              目标 CA <span className="text-danger">*</span>
            </Label>
            <Controller
              control={control}
              name="preset"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="le_staging">Let's Encrypt(测试 / Staging)</SelectItem>
                    <SelectItem value="le_prod">Let's Encrypt(生产)</SelectItem>
                    <SelectItem value="custom">自定义 ACME CA…</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
            <p className="text-xs text-muted-foreground">
              测试环境(Staging)用于验证流程、不产出受信任证书;正式签发选生产。
            </p>
          </div>

          {isCustom && (
            <>
              <div className="space-y-1.5">
                <Label htmlFor="directoryUrl">
                  ACME 目录 URL <span className="text-danger">*</span>
                </Label>
                <Input
                  id="directoryUrl"
                  placeholder="https://ca.example.com/acme/directory"
                  className="font-mono text-[13px]"
                  aria-invalid={!!errors.directoryUrl}
                  {...reg("directoryUrl")}
                />
                {errors.directoryUrl && (
                  <p className="text-xs text-danger">{errors.directoryUrl.message}</p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="caLabel">CA 展示名(可选)</Label>
                <Input id="caLabel" placeholder="如 ZeroSSL / 内部 CA" {...reg("caLabel")} />
              </div>
            </>
          )}

          <div className="space-y-1.5">
            <Label htmlFor="contactEmail">
              联系邮箱 <span className="text-danger">*</span>
            </Label>
            <Input
              id="contactEmail"
              type="email"
              placeholder="admin@example.com"
              className="font-mono text-[13px]"
              aria-invalid={!!errors.contactEmail}
              {...reg("contactEmail")}
            />
            {errors.contactEmail && (
              <p className="text-xs text-danger">{errors.contactEmail.message}</p>
            )}
            <p className="text-xs text-muted-foreground">CA 用于到期提醒等通知。</p>
          </div>

          <div className="space-y-1.5">
            <div className="flex items-start gap-2">
              <Controller
                control={control}
                name="tosAgreed"
                render={({ field }) => (
                  <Checkbox
                    id="tosAgreed"
                    checked={field.value}
                    onCheckedChange={field.onChange}
                    className="mt-0.5"
                    aria-label="同意服务条款"
                  />
                )}
              />
              <Label htmlFor="tosAgreed" className="cursor-pointer font-normal leading-snug">
                我已阅读并同意该 CA 的服务条款(ToS)
              </Label>
            </div>
            {errors.tosAgreed && <p className="text-xs text-danger">{errors.tosAgreed.message}</p>}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => close(false)}
              disabled={register.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={register.isPending}>
              {register.isPending && <Loader2 className="animate-spin" />}
              注册账户
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
