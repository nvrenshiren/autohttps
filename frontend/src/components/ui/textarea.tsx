import * as React from "react";
import { cn } from "@/lib/utils";

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "flex min-h-16 w-full rounded-md border border-input bg-background px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none md:text-sm",
        "placeholder:text-muted-foreground/80 hover:border-border-strong",
        "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        "aria-[invalid=true]:border-destructive aria-[invalid=true]:ring-destructive/30",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
