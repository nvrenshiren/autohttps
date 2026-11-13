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
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

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
  ["/", "/domains", "/certificates", "/tasks"].includes(item.to),
);

const NAV_GROUPS = [
  { label: "工作台", items: NAV.filter(({ to }) => ["/", "/tasks"].includes(to)) },
  { label: "证书资产", items: NAV.filter(({ to }) => ["/domains", "/certificates"].includes(to)) },
  { label: "签发基础设施", items: NAV.filter(({ to }) => ["/acme", "/local-ca"].includes(to)) },
  { label: "系统", items: NAV.filter(({ to }) => to === "/settings") },
];

function BrandMark({ className }: { className?: string }) {
  return <img src="/icon.svg" alt="" aria-hidden="true" className={cn("size-9 shrink-0 rounded-xl", className)} />;
}

function NotificationDot({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span className="absolute -right-1.5 -top-1.5 inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-notification px-1 text-[10px] font-semibold leading-none text-notification-foreground shadow-[0_0_0_2px_var(--color-sidebar)]">
      {count > 99 ? "99+" : count}
    </span>
  );
}

function Sidebar({ pendingCount, runModeLabel }: { pendingCount: number; runModeLabel: string }) {
  return (
    <aside className="hidden w-16 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground md:flex xl:w-60">
      <div className="flex h-16 shrink-0 items-center justify-center border-b border-sidebar-border px-3 xl:justify-start xl:px-4">
        <div className="flex min-w-0 items-center gap-3">
          <BrandMark className="size-10" />
          <div className="hidden min-w-0 xl:block">
            <div className="font-semibold leading-none tracking-tight text-foreground">autohttps</div>
            <div className="pt-1 text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground">
              Certificate operations
            </div>
          </div>
        </div>
      </div>

      <nav aria-label="主导航" className="flex-1 overflow-y-auto px-2 py-3">
        {NAV_GROUPS.map((group, groupIndex) => (
          <section key={group.label} aria-label={group.label} className="mb-2 last:mb-0">
            {groupIndex > 0 && (
              <span aria-hidden="true" className="mx-3 mb-2 mt-3 block h-px bg-sidebar-border xl:hidden" />
            )}
            <h2 className="hidden px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground xl:block">
              {group.label}
            </h2>
            <div className="space-y-1">
              {group.items.map(({ to, label, Icon, end }) => (
                <Tooltip key={to}>
                  <TooltipTrigger asChild>
                    <NavLink
                      to={to}
                      end={end}
                      aria-label={label}
                      className={({ isActive }) =>
                        cn(
                          "relative flex min-h-11 items-center gap-3 rounded-md px-2.5 text-sm transition-colors duration-150",
                          "justify-center xl:justify-start",
                          "focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-sidebar-ring focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar",
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
                              className={cn("size-5 shrink-0", isActive && "text-sidebar-primary")}
                              strokeWidth={isActive ? 2.1 : 1.8}
                            />
                            {to === "/" && <NotificationDot count={pendingCount} />}
                          </span>
                          <span className="hidden xl:inline">{label}</span>
                        </>
                      )}
                    </NavLink>
                  </TooltipTrigger>
                  <TooltipContent side="right" className="xl:hidden">
                    {label}
                  </TooltipContent>
                </Tooltip>
              ))}
            </div>
          </section>
        ))}
      </nav>

      <div className="shrink-0 border-t border-sidebar-border px-3 py-3">
        <div className="hidden items-center gap-2 rounded-md border border-sidebar-border bg-sidebar-accent/40 px-3 py-2 text-xs text-sidebar-foreground xl:flex">
          <Server className="size-3.5 shrink-0 text-sidebar-primary" aria-hidden="true" />
          <span>{runModeLabel}运行</span>
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
            "flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-lg text-xs font-medium text-muted-foreground transition-colors hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2",
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
            <BadgeCheck aria-hidden="true" />
            ACME 账户
          </NavLink>
        </DropdownMenuItem>
        <DropdownMenuItem asChild>
          <NavLink to="/local-ca" className="flex w-full items-center gap-2">
            <Landmark aria-hidden="true" />
            根 CA
          </NavLink>
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem asChild>
          <NavLink to="/settings" className="flex w-full items-center gap-2">
            <SettingsIcon aria-hidden="true" />
            设置
          </NavLink>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function MobileNavigation({ pathname }: { pathname: string }) {
  const moreActive =
    pathname.startsWith("/acme") ||
    pathname.startsWith("/local-ca") ||
    pathname.startsWith("/settings");

  return (
    <nav
      aria-label="移动端导航"
      className="fixed inset-x-0 bottom-0 z-30 flex min-h-16 items-center gap-1 border-t border-border bg-background px-2 pb-[env(safe-area-inset-bottom)] pt-1 md:hidden"
    >
      {MOBILE_NAV.map(({ to, label, Icon, end }) => (
        <NavLink
          key={to}
          to={to}
          end={end}
          aria-label={label}
          className={({ isActive }) =>
            cn(
              "flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-lg text-xs font-medium transition-colors hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2",
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
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-lg focus:bg-background focus:px-4 focus:py-3 focus:text-foreground focus:shadow-pop focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2"
      >
        跳到主要内容
      </a>
      <Sidebar pendingCount={pendingCount} runModeLabel={runModeLabel} />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <header className="relative z-20 flex h-14 shrink-0 items-center gap-3 border-b border-border bg-background px-4 sm:px-6">
          <span aria-hidden="true" className="absolute inset-x-0 top-0 h-px brand-gradient opacity-70" />
          <div className="flex items-center gap-2 md:hidden">
            <BrandMark className="size-8 rounded-lg" />
            <span className="font-semibold tracking-tight">autohttps</span>
          </div>
          <div className="hidden items-center gap-2 md:flex xl:hidden">
            <BrandMark className="size-8 rounded-lg" />
            <span className="text-xs font-medium tracking-wide text-muted-foreground">证书生命周期管理</span>
          </div>
          <span className="hidden text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground xl:block">
            Secure certificate operations
          </span>

          <div className="ml-auto flex items-center gap-2 sm:gap-3">
            <div className="hidden items-center gap-1.5 rounded-md border border-border bg-card px-2.5 py-1.5 text-xs text-muted-foreground md:flex xl:hidden">
              <Server className="size-3.5" aria-hidden="true" />
              <span>{runModeLabel}运行</span>
            </div>
            <div
              role="status"
              aria-live="polite"
              className={cn(
                "hidden items-center gap-2 rounded-md border px-2.5 py-1.5 text-xs lg:flex",
                connected
                  ? "border-success/25 bg-success-muted/60 text-success-muted-foreground"
                  : "border-info/25 bg-info-muted/60 text-info-muted-foreground",
              )}
            >
              <span className={cn("size-1.5 rounded-full", connected ? "pulse-dot bg-success" : "bg-info")} />
              {connected ? "实时同步" : "正在连接"}
            </div>
            <ThemeToggle />
          </div>
        </header>

        <main
          id="main-content"
          tabIndex={-1}
          className="relative isolate min-h-0 flex-1 overflow-auto overscroll-contain pb-[calc(5rem+env(safe-area-inset-bottom))] md:pb-0"
        >
          <div aria-hidden="true" className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[34rem] grid-paper opacity-80" />
          <div key={location.pathname} className="page-in min-h-full">
            {children}
          </div>
        </main>
      </div>
      <MobileNavigation pathname={location.pathname} />
    </div>
  );
}
