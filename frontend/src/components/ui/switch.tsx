import * as React from "react";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import { cn } from "@/lib/utils";

function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "group/switch relative inline-flex size-11 shrink-0 items-center justify-center rounded-full outline-none transition-colors",
        "focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <span
        aria-hidden="true"
        className={cn(
          "absolute left-1/2 top-1/2 h-5 w-9 -translate-x-1/2 -translate-y-1/2 rounded-full border border-transparent bg-input transition-colors",
          "group-data-[state=checked]/switch:bg-primary dark:group-data-[state=unchecked]/switch:bg-input/80",
        )}
      />
      <SwitchPrimitive.Thumb
        aria-hidden="true"
        className={cn(
          "pointer-events-none absolute left-1/2 top-1/2 z-10 block size-4 -translate-x-4 -translate-y-1/2 rounded-full bg-background shadow-sm ring-0 transition-transform",
          "group-data-[state=checked]/switch:translate-x-0 dark:group-data-[state=checked]/switch:bg-primary-foreground dark:group-data-[state=unchecked]/switch:bg-foreground",
        )}
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
