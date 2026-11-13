import { ChevronRight } from "lucide-react";
import { Link } from "react-router";

export interface Crumb {
  label: string;
  to?: string;
}

/** 页首:面包屑 + 标题 + 右侧操作区。详情页承载层级面包屑(设计 §7.6)。 */
export function PageHeader({
  title,
  description,
  crumbs,
  actions,
}: {
  title: string;
  description?: string;
  crumbs?: Crumb[];
  actions?: React.ReactNode;
}) {
  return (
    <div data-slot="page-header" className="mb-7 flex flex-col items-stretch gap-4 border-b border-border/70 pb-6 sm:flex-row sm:items-end sm:justify-between">
      <div className="min-w-0">
        <div className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.16em] text-primary">
          <span aria-hidden="true" className="size-1.5 rounded-full bg-primary" />
          <span>AutoHTTPS / Security Operations</span>
        </div>
        {crumbs && crumbs.length > 0 && (
          <nav aria-label="面包屑" className="mb-2 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
            {crumbs.map((c, i) => (
              <span
                key={i}
                aria-current={i === crumbs.length - 1 ? "page" : undefined}
                className="inline-flex items-center gap-1"
              >
                {i > 0 && <ChevronRight aria-hidden="true" className="size-3" />}
                {c.to ? (
                  <Link
                    to={c.to}
                    className="rounded-sm transition-colors hover:text-link focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring focus-visible:ring-offset-2"
                  >
                    {c.label}
                  </Link>
                ) : (
                  <span>{c.label}</span>
                )}
              </span>
            ))}
          </nav>
        )}
        <h1 className="max-w-full break-words font-display text-2xl font-semibold tracking-tight sm:text-3xl">{title}</h1>
        {description && (
          <p className="mt-2 max-w-2xl text-sm leading-relaxed text-muted-foreground">{description}</p>
        )}
      </div>
      {actions && <div className="flex w-full flex-wrap items-center gap-2 sm:w-auto sm:shrink-0 sm:justify-end">{actions}</div>}
    </div>
  );
}
