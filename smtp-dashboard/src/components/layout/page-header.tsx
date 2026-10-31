import type { ReactNode } from "react";

interface PageHeaderProps {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
  badge?: ReactNode;
  className?: string;
}

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
  badge,
  className = "",
}: PageHeaderProps) {
  return (
    <div
      className={`flex flex-col gap-4 pb-6 border-b border-zinc-200/80 md:flex-row md:items-center md:justify-between ${className}`}
    >
      <div className="space-y-1">
        {eyebrow && (
          <p className="text-[11px] font-bold tracking-wider uppercase text-emerald-700 select-none">
            {eyebrow}
          </p>
        )}
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-bold tracking-tight text-zinc-900">
            {title}
          </h1>
          {badge}
        </div>
        {description && (
          <p className="text-xs text-zinc-500 max-w-2xl leading-relaxed">
            {description}
          </p>
        )}
      </div>
      {actions && (
        <div className="flex shrink-0 items-center gap-2.5">{actions}</div>
      )}
    </div>
  );
}
