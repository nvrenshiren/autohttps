import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// 状态使用语义色；普通类别保持中性，并沿用设计系统的胶囊形标签。
const badgeVariants = cva(
  "inline-flex h-6 w-fit shrink-0 items-center justify-center gap-1 whitespace-nowrap rounded-full border px-2.5 text-xs font-medium [&>svg]:pointer-events-none [&>svg]:size-3",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        outline: "border-border-strong text-muted-foreground",
        "outline-warning": "border-warning/60 text-warning-muted-foreground",
        success: "border-success/20 bg-success-muted text-success-muted-foreground",
        warning: "border-warning/25 bg-warning-muted text-warning-muted-foreground",
        danger: "border-danger/20 bg-danger-muted text-danger-muted-foreground",
        info: "border-info/20 bg-info-muted text-info-muted-foreground",
        neutral: "border-border bg-neutral-muted text-neutral-muted-foreground",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

function Badge({
  className,
  variant,
  asChild = false,
  ...props
}: React.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : "span";
  return (
    <Comp data-slot="badge" className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

export { Badge, badgeVariants };
export type BadgeVariant = NonNullable<VariantProps<typeof badgeVariants>["variant"]>;
