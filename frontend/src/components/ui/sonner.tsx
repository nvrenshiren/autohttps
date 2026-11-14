import { Toaster as Sonner, type ToasterProps } from "sonner";

// 位置桌面右下(§7.7);使用不透明浮层 token。不承载表单校验错误(那走行内)。
function Toaster(props: ToasterProps) {
  return (
    <Sonner
      position="bottom-right"
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            "!bg-popover !rounded-xl !border-border-strong !shadow-pop",
        },
      }}
      style={
        {
          "--normal-bg": "var(--popover)",
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
