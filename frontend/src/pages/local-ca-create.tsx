/**
 * 新增根 CA(local-ca/create PRD)—— 二选一:创建(F2,name+validityDays → POST /root-cas)/ 导入
 * (F3,cert+key+口令 → POST /root-cas/import)。本地离线瞬时,无过渡态(LC1);失败留页提示原因(H3)。
 * 分段控件走「分段」范式(设计 §7.5,替代未引入的 RadioGroup)。校验错误行内,提交结果 toast。
 */
import { useState } from "react";
import { useNavigate } from "react-router";
import { Landmark, Loader2, Plus, Upload } from "lucide-react";
import { useCreateRootCa, useImportRootCa } from "@/lib/queries";
import { ApiError } from "@/lib/api";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/shared/page-header";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { toast } from "@/components/ui/sonner";

type Mode = "create" | "import";
type Errors = Partial<Record<"name" | "validityDays" | "certPem" | "privateKeyPem" | "keyPassphrase", string>>;

function FieldError({ msg }: { msg?: string }) {
  if (!msg) return null;
  return <p className="text-xs text-danger">{msg}</p>;
}

export function RootCaCreatePage() {
  const navigate = useNavigate();
  const create = useCreateRootCa();
  const importCa = useImportRootCa();
  const pending = create.isPending || importCa.isPending;

  const [mode, setMode] = useState<Mode>("create");
  const [name, setName] = useState("");
  const [validityDays, setValidityDays] = useState("3650");
  const [certPem, setCertPem] = useState("");
  const [privateKeyPem, setPrivateKeyPem] = useState("");
  const [keyPassphrase, setKeyPassphrase] = useState("");
  const [errors, setErrors] = useState<Errors>({});

  const switchMode = (m: Mode) => {
    if (m === mode) return;
    setMode(m);
    setErrors({});
  };

  const validate = (): Errors => {
    const e: Errors = {};
    if (!name.trim()) e.name = "请输入名称 / 标识";
    if (mode === "create") {
      const n = Number(validityDays);
      if (!validityDays.trim() || !Number.isFinite(n) || !Number.isInteger(n) || n <= 0) {
        e.validityDays = "有效期需为正整数(天)";
      }
    } else {
      if (!certPem.trim()) e.certPem = "请粘贴根 CA 证书 PEM";
      if (!privateKeyPem.trim()) e.privateKeyPem = "请粘贴配对私钥 PEM";
    }
    return e;
  };

  const onSubmit = (ev: React.FormEvent) => {
    ev.preventDefault();
    const e = validate();
    setErrors(e);
    if (Object.keys(e).length > 0) return;

    if (mode === "create") {
      create.mutate(
        { name: name.trim(), validityDays: Number(validityDays) },
        {
          onSuccess: (ca) => {
            toast.success(`根 CA「${ca.name}」已创建`);
            navigate(`/local-ca/${ca.id}`);
          },
          onError: (err) => {
            if (err instanceof ApiError) {
              if (err.code === "invalid_validity_period")
                setErrors({ validityDays: err.message });
              else if (err.code === "validation_failed") setErrors({ name: err.message });
              else toast.error(err.message);
            } else {
              toast.error("创建失败");
            }
          },
        },
      );
    } else {
      importCa.mutate(
        {
          name: name.trim(),
          certPem: certPem.trim(),
          privateKeyPem: privateKeyPem.trim(),
          keyPassphrase: keyPassphrase.trim() || undefined,
        },
        {
          onSuccess: (ca) => {
            toast.success(
              ca.status === "expired"
                ? `根 CA「${ca.name}」已导入(证书已过有效期)`
                : `根 CA「${ca.name}」已导入`,
            );
            navigate(`/local-ca/${ca.id}`);
          },
          onError: (err) => {
            if (err instanceof ApiError) {
              switch (err.code) {
                case "import_key_mismatch":
                  setErrors({ privateKeyPem: err.message });
                  break;
                case "import_invalid_certificate":
                  setErrors({ certPem: err.message });
                  break;
                case "import_key_decryption_failed":
                  setErrors({ keyPassphrase: err.message });
                  break;
                case "validation_failed":
                  setErrors({ name: err.message });
                  break;
                default:
                  toast.error(err.message);
              }
            } else {
              toast.error("导入失败");
            }
          },
        },
      );
    }
  };

  return (
    <div className="mx-auto max-w-2xl p-4 sm:p-6">
      <PageHeader
        title="新增根 CA"
        crumbs={[{ label: "根 CA", to: "/local-ca" }, { label: "新增" }]}
      />

      {/* 分段:创建 / 导入(二选一) */}
      <div className="mb-5 inline-flex rounded-lg border border-border bg-muted/40 p-1">
        {(
          [
            { m: "create" as const, label: "创建新根 CA", Icon: Plus },
            { m: "import" as const, label: "导入已有根 CA", Icon: Upload },
          ]
        ).map(({ m, label, Icon }) => (
          <button
            key={m}
            type="button"
            onClick={() => switchMode(m)}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
              mode === m
                ? "bg-background text-foreground shadow-xs"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="size-4" />
            {label}
          </button>
        ))}
      </div>

      <Alert className="mb-5">
        <Landmark />
        <AlertTitle>本地离线 · 瞬时完成</AlertTitle>
        <AlertDescription>
          创建 / 导入均为本地操作,无需外网,瞬时成功或失败(无「创建中 / 导入中」过渡态)。生成 / 导入的私钥为敏感数据,加密存储于数据目录,永不导出。
        </AlertDescription>
      </Alert>

      <Card>
        <CardContent className="py-6">
          <form onSubmit={onSubmit} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="ca-name">
                名称 / 标识 <span className="text-danger">*</span>
              </Label>
              <Input
                id="ca-name"
                placeholder="如 内网根 CA / Corp Root CA"
                value={name}
                aria-invalid={!!errors.name}
                onChange={(e) => setName(e.target.value)}
              />
              <FieldError msg={errors.name} />
            </div>

            {mode === "create" ? (
              <div className="space-y-1.5">
                <Label htmlFor="ca-validity">
                  有效期(天) <span className="text-danger">*</span>
                </Label>
                <Input
                  id="ca-validity"
                  type="number"
                  min={1}
                  className="max-w-40 font-mono text-[13px]"
                  value={validityDays}
                  aria-invalid={!!errors.validityDays}
                  onChange={(e) => setValidityDays(e.target.value)}
                />
                <FieldError msg={errors.validityDays} />
                <p className="text-xs text-muted-foreground">
                  自现在起的有效天数(如 3650 ≈ 10 年);到期后需创建 / 导入新根接替。密钥算法等技术参数由系统采用合理默认。
                </p>
              </div>
            ) : (
              <>
                <div className="space-y-1.5">
                  <Label htmlFor="ca-cert">
                    根 CA 证书(PEM) <span className="text-danger">*</span>
                  </Label>
                  <Textarea
                    id="ca-cert"
                    rows={6}
                    placeholder={"-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----"}
                    className="font-mono text-[12px]"
                    value={certPem}
                    aria-invalid={!!errors.certPem}
                    onChange={(e) => setCertPem(e.target.value)}
                  />
                  <FieldError msg={errors.certPem} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="ca-key">
                    配对私钥(PEM) <span className="text-danger">*</span>
                  </Label>
                  <Textarea
                    id="ca-key"
                    rows={6}
                    placeholder={"-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----"}
                    className="font-mono text-[12px]"
                    value={privateKeyPem}
                    aria-invalid={!!errors.privateKeyPem}
                    onChange={(e) => setPrivateKeyPem(e.target.value)}
                  />
                  <FieldError msg={errors.privateKeyPem} />
                  <p className="text-xs text-muted-foreground">
                    校验证书与私钥配对、且证书为可用根 CA;若证书本身已过期,导入后直接判为「已过期」。私钥为敏感数据,校验后加密落地。
                  </p>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="ca-pass">解密口令(私钥受口令保护时提供)</Label>
                  <Input
                    id="ca-pass"
                    type="password"
                    className="max-w-xs"
                    placeholder="未加密可留空"
                    value={keyPassphrase}
                    aria-invalid={!!errors.keyPassphrase}
                    onChange={(e) => setKeyPassphrase(e.target.value)}
                  />
                  <FieldError msg={errors.keyPassphrase} />
                </div>
              </>
            )}

            <div className="flex items-center justify-end gap-2 pt-2">
              <Button
                type="button"
                variant="outline"
                disabled={pending}
                onClick={() => navigate("/local-ca")}
              >
                取消
              </Button>
              <Button type="submit" disabled={pending}>
                {pending && <Loader2 className="animate-spin" />}
                {mode === "create" ? "创建" : "导入"}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
