import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// 状态 Badge:5 语义级(软底,§3.1 token);分类 Badge:outline 中性(§3.5,防彩虹汤)。
const badgeVariants = cva(
  "inline-flex items-center justify-center gap-1 rounded-md border px-2 h-5 text-xs font-medium w-fit whitespace-nowrap shrink-0 [&>svg]:size-3 [&>svg]:pointer-events-none",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        outline: "border-border text-muted-foreground",
        "outline-warning": "border-warning text-warning-muted-foreground",
        success: "border-transparent bg-success-muted text-success-muted-foreground",
        warning: "border-transparent bg-warning-muted text-warning-muted-foreground",
        danger: "border-transparent bg-danger-muted text-danger-muted-foreground",
        info: "border-transparent bg-info-muted text-info-muted-foreground",
        neutral: "border-transparent bg-neutral-muted text-neutral-muted-foreground",
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
