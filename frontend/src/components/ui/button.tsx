import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// 变体 → 操作语义：主操作使用生成的 COHESION 主色，危险操作使用独立实体危险色。
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors duration-150 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-xs hover:bg-primary-hover active:bg-primary-active",
        destructive:
          "bg-destructive-solid text-destructive-foreground shadow-xs hover:bg-destructive-hover active:bg-destructive-active",
        outline:
          "border border-border-strong bg-background shadow-xs hover:bg-accent hover:text-accent-foreground",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-accent hover:text-accent-foreground",
        link: "text-link underline-offset-4 hover:text-link-hover hover:underline active:text-link-active",
      },
      size: {
        default: "min-h-11 px-4 py-2 md:h-9 md:min-h-9 has-[>svg]:px-3",
        sm: "min-h-11 gap-1.5 rounded-md px-3 text-[13px] md:h-8 md:min-h-8",
        lg: "min-h-11 rounded-lg px-6 md:h-10 md:min-h-10",
        icon: "size-11 md:size-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size }), className)}
      {...props}
    />
  );
}

export { Button, buttonVariants };
