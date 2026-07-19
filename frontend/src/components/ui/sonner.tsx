import { Toaster as Sonner, type ToasterProps } from "sonner";

// 位置桌面右下(§7.7);颜色走 token,玻璃浮层。不承载表单校验错误(那走行内)。
function Toaster(props: ToasterProps) {
  return (
    <Sonner
      position="bottom-right"
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            "glass !rounded-xl !border-border-strong !shadow-pop",
        },
      }}
      style={
        {
          "--normal-bg": "color-mix(in oklch, var(--popover) 88%, transparent)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border-strong)",
        } as React.CSSProperties
      }
      {...props}
    />
  );
}

export { Toaster };
export { toast } from "sonner";
