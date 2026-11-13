import * as React from "react";
import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "flex h-11 w-full min-w-0 rounded-md border border-input bg-background px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none md:h-9 md:text-sm",
        "placeholder:text-muted-foreground/80 file:border-0 file:bg-transparent file:text-sm file:font-medium",
        "hover:border-border-strong focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        "aria-[invalid=true]:border-destructive aria-[invalid=true]:ring-destructive/30",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
