/**
 * App Shell(设计系统 v2「极光守护」)—— 800×600 响应式:**56px 图标轨为默认**,
 * `≥1024`(lg)展开 240px 标签侧栏。品牌标为极光渐变锁;激活项为玻璃胶囊 + 品牌描边 + 微光。
 * 红点仅挂总览(dashboard 唯一落点)。运行形态只读 chip + 主题切换。环境光由 .ambient 承载。
 */
import { NavLink, useLocation } from "react-router";
import {
  BadgeCheck,
  Globe,
  Landmark,
  LayoutDashboard,
  ListChecks,
  Settings as SettingsIcon,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useAppInfo, useDashboard } from "@/lib/queries";
import { ThemeToggle } from "@/components/theme-toggle";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface NavItem {
  to: string;
  label: string;
  Icon: LucideIcon;
  end?: boolean;
}

// 7 导航项(顺序 + 图标立法,§9.1)
const NAV: NavItem[] = [
  { to: "/", label: "总览", Icon: LayoutDashboard, end: true },
  { to: "/domains", label: "域名", Icon: Globe },
  { to: "/certificates", label: "证书", Icon: ShieldCheck },
  { to: "/acme", label: "ACME", Icon: BadgeCheck },
  { to: "/local-ca", label: "根 CA", Icon: Landmark },
  { to: "/tasks", label: "任务", Icon: ListChecks },
  { to: "/settings", label: "设置", Icon: SettingsIcon },
];

function sectionTitle(pathname: string): string {
  if (pathname === "/") return "总览";
  const seg = "/" + pathname.split("/")[1];
  return NAV.find((n) => n.to === seg)?.label ?? "autohttps";
}

function BrandMark({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "brand-gradient inline-flex shrink-0 items-center justify-center rounded-xl text-primary-foreground",
        "shadow-[0_0_0_1px_oklch(1_0_0/18%)_inset,0_4px_14px_-2px_oklch(0.52_0.21_267/50%)]",
        className,
      )}
    >
      <ShieldCheck className="size-[62%]" strokeWidth={2.2} />
    </span>
  );
}

function NotificationDot({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span className="absolute -right-1.5 -top-1.5 inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-notification px-1 text-[10px] font-semibold leading-none text-notification-foreground shadow-[0_0_0_2px_var(--color-sidebar)]">
      {count > 99 ? "99+" : count}
    </span>
  );
}

function Sidebar({ pendingCount }: { pendingCount: number }) {
  const appInfo = useAppInfo();
  const runModeLabel = appInfo.data?.runMode === "desktop" ? "桌面" : "服务器";

  return (
    <aside className="flex w-14 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground lg:w-60">
      {/* 品牌 */}
      <div className="flex h-14 shrink-0 items-center gap-2.5 border-b border-sidebar-border px-3 lg:px-4">
        <BrandMark className="size-7" />
        <span className="hidden font-display text-[15px] font-semibold tracking-tight lg:inline">
          auto<span className="text-aurora">https</span>
        </span>
      </div>

      {/* 导航 */}
      <nav className="flex-1 space-y-1 overflow-auto p-2">
        {NAV.map(({ to, label, Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) =>
              cn(
                "relative flex h-9 items-center gap-2.5 rounded-lg px-2.5 transition-all duration-150",
                "justify-center lg:justify-start",
                isActive
                  ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground shadow-[inset_0_0_0_1px_var(--color-border-strong)]"
                  : "text-sidebar-foreground/70 hover:bg-sidebar-accent/70 hover:text-sidebar-accent-foreground",
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <span className="brand-gradient absolute left-0 top-2 bottom-2 w-[3px] rounded-full shadow-[0_0_8px_oklch(0.52_0.21_267/60%)]" />
                )}
                <span className="relative inline-flex items-center justify-center">
                  <Icon
                    className={cn(
                      "size-5 transition-colors",
                      isActive && "text-sidebar-primary",
                    )}
                  />
                  {to === "/" && <NotificationDot count={pendingCount} />}
                </span>
                <span className="hidden lg:inline">{label}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>

      {/* 运行形态只读 chip(取自 API,TECH §3.6) */}
      <div className="shrink-0 border-t border-sidebar-border p-3">
        <Tooltip>
          <TooltipTrigger asChild>
            <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground lg:justify-start">
              <span className="relative inline-flex size-1.5 shrink-0">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-success opacity-60" />
                <span className="relative inline-flex size-1.5 rounded-full bg-success" />
              </span>
              <span className="hidden lg:inline">运行形态:{runModeLabel}</span>
            </div>
          </TooltipTrigger>
          <TooltipContent side="right">运行形态:{runModeLabel}(只读)</TooltipContent>
        </Tooltip>
      </div>
    </aside>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const dashboard = useDashboard();
  const pendingCount = dashboard.data?.pendingCount ?? 0;

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <div className="ambient" aria-hidden />
      <Sidebar pendingCount={pendingCount} />
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border px-4 sm:px-6">
          <span className="font-display text-base font-semibold tracking-tight">
            {sectionTitle(location.pathname)}
          </span>
          <div className="ml-auto flex items-center gap-1">
            <ThemeToggle />
          </div>
        </header>
        <main className="flex-1 overflow-auto">
          <div key={location.pathname} className="page-in h-full">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
