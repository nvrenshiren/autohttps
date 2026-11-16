import { Link, NavLink, useLocation } from "react-router";
import {
  ChevronDown,
  Ellipsis,
  Globe,
  KeyRound,
  Landmark,
  LayoutDashboard,
  ListChecks,
  Server,
  Settings as SettingsIcon,
  ShieldCheck,
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

function BrandMark({ className }: { className?: string }) {
  return <img src="/icon.svg" alt="" aria-hidden="true" className={cn("size-8 shrink-0", className)} />;
}

function NotificationDot({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span className="absolute -right-2 -top-1 inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-notification px-1 text-[10px] font-semibold leading-none text-notification-foreground ring-2 ring-background">
      {count > 99 ? "99+" : count}
    </span>
  );
}

function RouteItem({ to, children, end = false }: { to: string; children: React.ReactNode; end?: boolean }) {
  return (
    <DropdownMenuItem asChild>
      <NavLink to={to} end={end} className="flex min-h-10 items-center gap-2.5">
        {children}
      </NavLink>
    </DropdownMenuItem>
  );
}

function TopNavigation({ pathname, pendingCount }: { pathname: string; pendingCount: number }) {
  const issueActive = pathname.includes("/issue") || pathname.includes("/challenges");
  const assetActive = pathname.startsWith("/domains") || (pathname.startsWith("/certificates") && !issueActive);
  const sourceActive = pathname.startsWith("/acme") || pathname.startsWith("/local-ca");

  const linkClass = ({ isActive }: { isActive: boolean }) =>
    cn(
      "inline-flex min-h-9 items-center gap-2 rounded-md px-2.5 text-[13px] font-medium transition-colors duration-150 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
      isActive ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
    );

  return (
    <nav aria-label="主导航" className="hidden min-w-0 items-center gap-1 lg:flex">
      <NavLink to="/" end aria-label={pendingCount > 0 ? `概览，有 ${pendingCount} 项待处理` : "概览"} className={linkClass}>
        <span className="relative"><LayoutDashboard aria-hidden="true" className="size-4" /><NotificationDot count={pendingCount} /></span>
        <span>概览</span>
      </NavLink>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className={cn(linkClass({ isActive: assetActive }), "group")}>
            <ShieldCheck aria-hidden="true" className="size-4" />
            <span>资产</span>
            <ChevronDown aria-hidden="true" className="size-3.5 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="min-w-48">
          <RouteItem to="/domains"><Globe aria-hidden="true" className="size-4 text-muted-foreground" />域名</RouteItem>
          <RouteItem to="/certificates"><ShieldCheck aria-hidden="true" className="size-4 text-muted-foreground" />证书</RouteItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Link to="/certificates/issue" aria-current={issueActive ? "page" : undefined} className={linkClass({ isActive: issueActive })}>
        <KeyRound aria-hidden="true" className="size-4" />
        <span>发起签发</span>
      </Link>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" className={cn(linkClass({ isActive: sourceActive }), "group")}>
            <Landmark aria-hidden="true" className="size-4" />
            <span>签发来源</span>
            <ChevronDown aria-hidden="true" className="size-3.5 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="min-w-52">
          <RouteItem to="/acme"><KeyRound aria-hidden="true" className="size-4 text-muted-foreground" />ACME 账户</RouteItem>
          <RouteItem to="/local-ca"><Landmark aria-hidden="true" className="size-4 text-muted-foreground" />本地根 CA</RouteItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <NavLink to="/tasks" className={linkClass}>
        <ListChecks aria-hidden="true" className="size-4" />
        <span>执行记录</span>
      </NavLink>
      <NavLink to="/settings" className={linkClass}>
        <SettingsIcon aria-hidden="true" className="size-4" />
        <span>系统</span>
      </NavLink>
    </nav>
  );
}

function CompactNavigation({ pendingCount }: { pendingCount: number }) {
  return (
    <div className="hidden items-center md:flex lg:hidden">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button type="button" aria-label="打开主导航" className="inline-flex min-h-9 items-center gap-2 rounded-md px-2.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background">
            <Ellipsis aria-hidden="true" className="size-4" />
            <span>导航</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="min-w-56">
          <RouteItem to="/" end><span className="relative"><LayoutDashboard aria-hidden="true" /><NotificationDot count={pendingCount} /></span>概览</RouteItem>
          <DropdownMenuSeparator />
          <RouteItem to="/domains"><Globe aria-hidden="true" className="text-muted-foreground" />域名</RouteItem>
          <RouteItem to="/certificates"><ShieldCheck aria-hidden="true" className="text-muted-foreground" />证书</RouteItem>
          <RouteItem to="/certificates/issue"><KeyRound aria-hidden="true" className="text-muted-foreground" />发起签发</RouteItem>
          <DropdownMenuSeparator />
          <RouteItem to="/acme"><KeyRound aria-hidden="true" className="text-muted-foreground" />ACME 账户</RouteItem>
          <RouteItem to="/local-ca"><Landmark aria-hidden="true" className="text-muted-foreground" />本地根 CA</RouteItem>
          <DropdownMenuSeparator />
          <RouteItem to="/tasks"><ListChecks aria-hidden="true" className="text-muted-foreground" />执行记录</RouteItem>
          <RouteItem to="/settings"><SettingsIcon aria-hidden="true" className="text-muted-foreground" />系统设置与备份</RouteItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

function MobileMoreMenu({ active }: { active: boolean }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label="更多导航"
          className={cn(
            "flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-md text-xs font-medium text-muted-foreground transition-colors hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2",
            active && "bg-accent text-accent-foreground",
          )}
        >
          <Ellipsis aria-hidden="true" className="size-[18px]" />
          <span>更多</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" side="top" className="mb-2 min-w-52">
        <DropdownMenuItem asChild><NavLink to="/certificates/issue" className="flex w-full items-center gap-2"><KeyRound aria-hidden="true" />发起签发</NavLink></DropdownMenuItem>
        <DropdownMenuItem asChild><NavLink to="/acme" className="flex w-full items-center gap-2"><KeyRound aria-hidden="true" />ACME 账户</NavLink></DropdownMenuItem>
        <DropdownMenuItem asChild><NavLink to="/local-ca" className="flex w-full items-center gap-2"><Landmark aria-hidden="true" />本地根 CA</NavLink></DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem asChild><NavLink to="/settings" className="flex w-full items-center gap-2"><SettingsIcon aria-hidden="true" />系统设置与备份</NavLink></DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function MobileNavigation({ pathname, pendingCount }: { pathname: string; pendingCount: number }) {
  const moreActive = pathname.includes("/issue") || pathname.includes("/challenges") || pathname.startsWith("/acme") || pathname.startsWith("/local-ca") || pathname.startsWith("/settings");
  const itemClass = (isActive: boolean) =>
    cn(
      "relative flex min-h-12 min-w-0 flex-1 flex-col items-center justify-center gap-0.5 rounded-md text-xs font-medium transition-colors hover:bg-accent/70 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2",
      isActive ? "bg-accent text-accent-foreground" : "text-muted-foreground",
    );
  const certificateActive = pathname.startsWith("/certificates") && !pathname.includes("/issue") && !pathname.includes("/challenges");

  return (
    <nav aria-label="移动端导航" className="fixed inset-x-0 bottom-0 z-30 flex min-h-16 items-center gap-1 border-t border-border bg-background/95 px-2 pb-[env(safe-area-inset-bottom)] pt-1 backdrop-blur-sm md:hidden">
      <Link to="/" aria-current={pathname === "/" ? "page" : undefined} aria-label={pendingCount > 0 ? `概览，有 ${pendingCount} 项待处理` : "概览"} className={itemClass(pathname === "/")}><span className="relative"><LayoutDashboard aria-hidden="true" className="size-[18px]" /><NotificationDot count={pendingCount} /></span><span>概览</span></Link>
      <Link to="/domains" aria-current={pathname.startsWith("/domains") ? "page" : undefined} aria-label="域名" className={itemClass(pathname.startsWith("/domains"))}><Globe aria-hidden="true" className="size-[18px]" /><span>域名</span></Link>
      <Link to="/certificates" aria-current={certificateActive ? "page" : undefined} aria-label="证书" className={itemClass(certificateActive)}><ShieldCheck aria-hidden="true" className="size-[18px]" /><span>证书</span></Link>
      <Link to="/tasks" aria-current={pathname.startsWith("/tasks") ? "page" : undefined} aria-label="执行记录" className={itemClass(pathname.startsWith("/tasks"))}><ListChecks aria-hidden="true" className="size-[18px]" /><span>记录</span></Link>
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
  const runModeLabel = appInfo.data?.runMode === "desktop" ? "桌面实例" : appInfo.data?.runMode === "server" ? "服务器实例" : "当前实例";

  return (
    <div className="app-frame flex h-dvh min-h-0 flex-col bg-background text-foreground">
      <a href="#main-content" className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-4 focus:py-3 focus:text-foreground focus:shadow-pop focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2">
        跳到主要内容
      </a>

      <header className="app-topbar relative z-20 flex min-h-14 shrink-0 items-center gap-3 border-b border-border bg-background px-3 sm:px-5 lg:px-7">
        <span aria-hidden="true" className="absolute inset-x-0 top-0 h-px brand-gradient opacity-80" />
        <NavLink to="/" aria-label="autohttps 概览" className="flex shrink-0 items-center gap-2 rounded-sm focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2">
          <BrandMark />
          <span className="hidden font-semibold tracking-tight sm:inline">autohttps</span>
        </NavLink>

        <div className="hidden h-6 w-px bg-border md:block" aria-hidden="true" />
        <TopNavigation pathname={location.pathname} pendingCount={pendingCount} />
        <CompactNavigation pendingCount={pendingCount} />

        <div className="ml-auto flex shrink-0 items-center gap-2 sm:gap-3">
          <div role="status" aria-live="polite" className={cn("inline-flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs", connected ? "border-success/25 bg-success-muted/60 text-success-muted-foreground" : "border-info/25 bg-info-muted/60 text-info-muted-foreground")}>
            <span aria-hidden="true" className={cn("size-1.5 rounded-full", connected ? "bg-success" : "bg-info")} />
            <span className="hidden sm:inline">{connected ? "实时连接" : "连接中"}</span>
          </div>
          <span className="hidden items-center gap-1.5 text-xs text-muted-foreground lg:inline-flex">
            <Server aria-hidden="true" className="size-3.5" />{runModeLabel}
          </span>
          <div className="hidden h-5 w-px bg-border lg:block" aria-hidden="true" />
          <ThemeToggle />
        </div>
      </header>

      <main id="main-content" tabIndex={-1} className="app-canvas relative isolate min-h-0 flex-1 overflow-auto overscroll-contain pb-[calc(5rem+env(safe-area-inset-bottom))] md:pb-0">
        <div aria-hidden="true" className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-80 grid-paper opacity-40" />
        <div key={location.pathname} className="page-in min-h-full">
          {children}
        </div>
      </main>
      <MobileNavigation pathname={location.pathname} pendingCount={pendingCount} />
      <div className="sr-only" aria-live="polite">{pendingCount > 0 ? `有 ${pendingCount} 项证书待处理` : "没有待处理证书"}</div>
    </div>
  );
}
