/**
 * 新增域名对话框(domains B1)—— react-hook-form + zod(表单栈);校验错误走行内(§7.5),
 * 成功/服务端失败走 toast。API 错误码映射到字段错误(domain_already_exists / wildcard_requires_dns01)。
 */
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Loader2 } from "lucide-react";
import { useCreateDomain } from "@/lib/queries";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { toast } from "@/components/ui/sonner";

const schema = z
  .object({
    hostname: z
      .string()
      .min(1, "请输入 hostname")
      .regex(/^(\*\.)?([a-zA-Z0-9-]+\.)+[a-zA-Z0-9-]+$/, "hostname 格式非法(如 example.com / *.example.com)"),
    groupName: z.string().optional(),
    remark: z.string().optional(),
    validationMethod: z.enum(["http_01", "dns_01"]).optional(),
  })
  .refine((d) => !(d.hostname.startsWith("*.") && d.validationMethod === "http_01"), {
    message: "通配符域名的验证方式必须为 dns_01",
    path: ["validationMethod"],
  });

type FormValues = z.infer<typeof schema>;

export function CreateDomainDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const create = useCreateDomain();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { hostname: "", groupName: "", remark: "" },
  });
  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors },
  } = form;

  const onSubmit = handleSubmit((values) => {
    create.mutate(
      {
        hostname: values.hostname.trim(),
        groupName: values.groupName?.trim() || undefined,
        remark: values.remark?.trim() || undefined,
        validationMethod: values.validationMethod,
      },
      {
        onSuccess: () => {
          toast.success("域名已新增");
          reset();
          onOpenChange(false);
        },
        onError: (e) => {
          if (e instanceof ApiError) {
            if (e.code === "domain_already_exists") setError("hostname", { message: e.message });
            else if (e.code === "wildcard_requires_dns01")
              setError("validationMethod", { message: e.message });
            else if (e.code === "validation_failed") setError("hostname", { message: e.message });
            else toast.error(e.message);
          } else {
            toast.error("新增失败");
          }
        },
      },
    );
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>新增域名</DialogTitle>
          <DialogDescription>hostname 为身份、创建后不可改(改名 = 删 + 增)。</DialogDescription>
        </DialogHeader>

        <form onSubmit={onSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="hostname">
              hostname <span className="text-danger">*</span>
            </Label>
            <Input
              id="hostname"
              placeholder="example.com 或 *.example.com"
              className="font-mono text-[13px]"
              aria-invalid={!!errors.hostname}
              {...register("hostname")}
            />
            {errors.hostname && (
              <p className="text-xs text-danger">{errors.hostname.message}</p>
            )}
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="groupName">分组(可选)</Label>
            <Input id="groupName" placeholder="如 prod / internal" {...register("groupName")} />
          </div>

          <div className="space-y-1.5">
            <Label>验证方式(可选)</Label>
            <Controller
              control={control}
              name="validationMethod"
              render={({ field }) => (
                <Select value={field.value} onValueChange={field.onChange}>
                  <SelectTrigger className="w-full">
                    <SelectValue placeholder="未设置" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="http_01">HTTP-01(webroot)</SelectItem>
                    <SelectItem value="dns_01">DNS-01(手动)</SelectItem>
                  </SelectContent>
                </Select>
              )}
            />
            {errors.validationMethod && (
              <p className="text-xs text-danger">{errors.validationMethod.message}</p>
            )}
            <p className="text-xs text-muted-foreground">通配符域名(*.）必须使用 DNS-01。</p>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="remark">备注(可选)</Label>
            <Input id="remark" placeholder="备注信息" {...register("remark")} />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={create.isPending}
            >
              取消
            </Button>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending && <Loader2 className="animate-spin" />}
              新增
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
