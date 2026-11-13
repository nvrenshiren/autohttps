import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Checkbox with a native input and an expanded 44px hit area for touch.
 * Keep the visual indicator at the shadcn 16px size.
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
    <span
      className={cn(
        "relative inline-flex size-11 shrink-0 items-center justify-center rounded-md transition-colors",
        !disabled && "hover:bg-accent/50",
        className,
      )}
    >
      <input
        type="checkbox"
        id={id}
        checked={checked}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledby}
        aria-invalid={ariaInvalid}
        aria-describedby={ariaDescribedby}
        data-slot="checkbox"
        onChange={(event) => onCheckedChange(event.currentTarget.checked)}
        className="peer absolute inset-0 z-10 size-full cursor-pointer appearance-none rounded-md outline-none disabled:cursor-not-allowed"
      />
      <span
        aria-hidden="true"
        className={cn(
          "inline-flex size-4 items-center justify-center rounded-[4px] border border-input bg-background shadow-xs transition-colors",
          "peer-checked:border-primary peer-checked:bg-primary peer-checked:text-primary-foreground",
          "peer-focus-visible:border-ring peer-focus-visible:ring-[3px] peer-focus-visible:ring-ring peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background",
          "peer-aria-[invalid=true]:border-destructive peer-aria-[invalid=true]:ring-destructive/20",
          "peer-disabled:opacity-50",
        )}
      >
        {checked && <Check aria-hidden="true" className="size-3.5" />}
      </span>
    </span>
  );
}
