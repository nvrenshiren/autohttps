import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Checkbox(设计 §7.5:同意服务条款 / 高风险确认勾选)。轻量实现(button role=checkbox),
 * 遵 v4 / React 19 约定(data-slot / data-state、无 forwardRef);键盘可达经 <button> 原生 Space/Enter。
 * 与 Switch 同 API(checked / onCheckedChange),区别在语义:勾选(tick)非开关(toggle)。
 */
export function Checkbox({
  checked,
  onCheckedChange,
  id,
  disabled,
  className,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledby,
  "aria-invalid": ariaInvalid,
  "aria-describedby": ariaDescribedby,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  id?: string;
  disabled?: boolean;
  className?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-invalid"?: boolean;
  "aria-describedby"?: string;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      id={id}
      aria-checked={checked}
      aria-label={ariaLabel}
      aria-labelledby={ariaLabelledby}
      aria-invalid={ariaInvalid}
      aria-describedby={ariaDescribedby}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
      data-slot="checkbox"
      data-state={checked ? "checked" : "unchecked"}
      className={cn(
        "peer inline-flex size-11 shrink-0 items-center justify-center rounded-md outline-none transition-colors hover:bg-accent/50",
        "focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "inline-flex size-4 items-center justify-center rounded-[4px] border shadow-xs transition-colors",
          checked
            ? "border-primary bg-primary text-primary-foreground"
            : "border-input bg-background",
        )}
      >
        {checked && <Check className="size-3.5" />}
      </span>
    </button>
  );
}
