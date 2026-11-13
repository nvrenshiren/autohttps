import { NavLink, useLocation } from "react-router";
import {
  BadgeCheck,
  Ellipsis,
  Globe,
  Landmark,
  LayoutDashboard,
  ListChecks,
  Server,
  Settings as SettingsIcon,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useAppInfo, useDashboard } from "@/lib/queries";
import { useUiStore } from "@/stores/ui";
import { ThemeToggle } from "@/components/theme-toggle";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface NavItem {
  to: string;
  label: string;
  Icon: LucideIcon;
  end?: boolean;
}

const NAV: NavItem[] = [
  { to: "/", label: "总览", Icon: LayoutDashboard, end: true },
  { to: "/domains", label: "域名", Icon: Globe },
  { to: "/certificates", label: "证书", Icon: ShieldCheck },
  { to: "/acme", label: "ACME", Icon: BadgeCheck },
  { to: "/local-ca", label: "根 CA", Icon: Landmark },
  { to: "/tasks", label: "任务", Icon: ListChecks },
  { to: "/settings", label: "设置", Icon: SettingsIcon },
];

const MOBILE_NAV = NAV.filter((item) =>
  ["/", "/domains", "/certificates", "/tasks", "/settings"].includes(item.to),
);

function BrandMark() {
  return (
    <span className="inline-flex size-9 shrink-0 items-center justify-center rounded-xl border border-primary/10 bg-accent text-primary">
      <ShieldCheck className="size-[18px]" strokeWidth={2} />
    </span>
  );
}

function BrandLogo({ className }: { className: string }) {
  return (
    <>
      <img
        src="/cohesion-logo-color.png"
        alt="COHESION 逐科"
        className={cn("dark:hidden", className)}
      />
      <img
        src="/cohesion-logo-white.png"
        alt="COHESION 逐科"
        className={cn("hidden dark:block", className)}
      />
    </>
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
  return (
    <aside className="hidden w-16 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground md:flex xl:w-60">
      <div className="flex h-16 shrink-0 items-center justify-center border-b border-sidebar-border px-3 xl:justify-start xl:px-4">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-primary/10 bg-accent text-primary xl:hidden">
          <ShieldCheck className="size-[18px]" strokeWidth={2} />
        </span>
        <div className="hidden min-w-0 items-center gap-3 xl:flex">
          <BrandMark />
          <div className="min-w-0">
            <div className="font-semibold leading-none tracking-tight text-foreground">autohttps</div>
            <BrandLogo className="mt-1 h-5 w-[86px] object-cover object-center" />
          </div>
        </div>
      </div>

      <nav aria-label="主导航" className="flex-1 space-y-1 overflow-y-auto p-2">
        {NAV.map(({ to, label, Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            aria-label={label}
            title={label}
            className={({ isActive }) =>
              cn(
                "relative flex h-10 items-center gap-3 rounded-lg px-2.5 text-sm transition-colors duration-150",
                "justify-center xl:justify-start",
                isActive
                  ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                  : "text-sidebar-foreground/75 hover:bg-sidebar-accent/70 hover:text-sidebar-accent-foreground",
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <span className="absolute bottom-2 left-0 top-2 w-[3px] rounded-r-full bg-sidebar-primary" />
                )}
                <span className="relative inline-flex items-center justify-center">
                  <Icon
                    aria-hidden="true"
                    className={cn("size-[18px] shrink-0", isActive && "text-sidebar-primary")}
                    strokeWidth={isActive ? 2.1 : 1.8}
                  />
                  {to === "/" && <NotificationDot count={pendingCount} />}
                </span>
                <span className="hidden xl:inline">{label}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="shrink-0 border-t border-sidebar-border px-3 py-4">
        <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground xl:justify-start">
          <Server className="size-3.5 shrink-0" aria-hidden="true" />
          <span className="hidden xl:inline">证书服务已启动</span>
        </div>
      </div>
    </aside>
  );
}

function MobileMoreMenu({ active }: { active: boolean }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label="更多导航"
          aria-current={active ? "page" : undefined}
          className={cn(
            "flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-lg text-[10px] font-medium text-muted-foreground transition-colors",
            active && "bg-accent text-accent-foreground",
          )}
        >
          <Ellipsis className="size-[18px]" aria-hidden="true" />
          <span>更多</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" side="top" className="mb-2 min-w-48">
        <DropdownMenuItem asChild>
          <NavLink to="/acme" className="flex w-full items-center gap-2">
            <BadgeCheck />
            ACME 账户
          </NavLink>
        </DropdownMenuItem>
        <DropdownMenuItem asChild>
          <NavLink to="/local-ca" className="flex w-full items-center gap-2">
            <Landmark />
            根 CA
          </NavLink>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function MobileNavigation({ pathname }: { pathname: string }) {
  const moreActive = pathname.startsWith("/acme") || pathname.startsWith("/local-ca");

  return (
    <nav
      aria-label="移动端导航"
      className="fixed inset-x-0 bottom-0 z-30 flex min-h-16 items-center gap-1 border-t border-border bg-background/95 px-2 pb-[env(safe-area-inset-bottom)] pt-1 shadow-[0_-4px_18px_-16px_rgb(0_0_0/28%)] backdrop-blur-md md:hidden"
    >
      {MOBILE_NAV.map(({ to, label, Icon, end }) => (
        <NavLink
          key={to}
          to={to}
          end={end}
          aria-label={label}
          className={({ isActive }) =>
            cn(
              "flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-lg text-[10px] font-medium transition-colors",
              isActive ? "bg-accent text-accent-foreground" : "text-muted-foreground",
            )
          }
        >
          <Icon className="size-[18px]" aria-hidden="true" />
          <span>{label}</span>
        </NavLink>
      ))}
      <MobileMoreMenu active={moreActive} />
    </nav>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const dashboard = useDashboard();
  const appInfo = useAppInfo();
  const connected = useUiStore((state) => state.sseConnected);
  const pendingCount = dashboard.data?.pendingCount ?? 0;
  const runModeLabel = appInfo.data?.runMode === "desktop" ? "桌面" : "服务器";

  return (
    <div className="flex h-dvh min-h-0 bg-transparent text-foreground">
      <Sidebar pendingCount={pendingCount} />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <header className="z-20 flex h-14 shrink-0 items-center gap-3 border-b border-border bg-background/90 px-4 backdrop-blur-md sm:px-6">
          <div className="flex items-center gap-2 md:hidden">
            <BrandLogo className="h-7 w-14 object-cover object-center" />
            <span className="font-semibold tracking-tight">autohttps</span>
          </div>
          <div className="hidden items-center gap-2 md:flex xl:hidden">
            <BrandLogo className="h-8 w-16 object-cover object-center" />
            <span className="text-xs text-muted-foreground">证书生命周期管理</span>
          </div>
          <span className="hidden text-sm text-muted-foreground xl:block">证书生命周期管理</span>

          <div className="ml-auto flex items-center gap-2 sm:gap-3">
            <div className="hidden items-center gap-1.5 rounded-full border border-border px-2.5 py-1 text-xs text-muted-foreground sm:flex">
              <Server className="size-3.5" aria-hidden="true" />
              <span>{runModeLabel}运行</span>
            </div>
            <div
              role="status"
              aria-live="polite"
              className="hidden items-center gap-1.5 text-xs text-muted-foreground lg:flex"
            >
              <span className={cn("size-1.5 rounded-full", connected ? "bg-success" : "bg-muted-foreground/50")} />
              {connected ? "实时同步" : "正在连接"}
            </div>
            <ThemeToggle />
          </div>
        </header>

        <main id="main-content" className="min-h-0 flex-1 overflow-auto overscroll-contain pb-20 md:pb-0">
          <div key={location.pathname} className="page-in min-h-full">
            {children}
          </div>
        </main>
      </div>
      <MobileNavigation pathname={location.pathname} />
    </div>
  );
}
