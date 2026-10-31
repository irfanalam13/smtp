import type { ReactNode } from "react";

export type StatTone = "green" | "blue" | "violet" | "amber" | "rose";

interface StatCardProps {
  label: string;
  value: string;
  detail: string;
  tone?: StatTone;
  icon?: ReactNode;
  trend?: {
    value: string;
    isPositive?: boolean;
  };
  className?: string;
}

const toneStyles: Record<
  StatTone,
  { bg: string; text: string; ring: string; border: string }
> = {
  green: {
    bg: "bg-emerald-50 dark:bg-emerald-950/30",
    text: "text-emerald-600 dark:text-emerald-400",
    ring: "ring-emerald-500/20",
    border: "border-emerald-100",
  },
  blue: {
    bg: "bg-sky-50 dark:bg-sky-950/30",
    text: "text-sky-600 dark:text-sky-400",
    ring: "ring-sky-500/20",
    border: "border-sky-100",
  },
  violet: {
    bg: "bg-purple-50 dark:bg-purple-950/30",
    text: "text-purple-600 dark:text-purple-400",
    ring: "ring-purple-500/20",
    border: "border-purple-100",
  },
  amber: {
    bg: "bg-amber-50 dark:bg-amber-950/30",
    text: "text-amber-600 dark:text-amber-400",
    ring: "ring-amber-500/20",
    border: "border-amber-100",
  },
  rose: {
    bg: "bg-rose-50 dark:bg-rose-950/30",
    text: "text-rose-600 dark:text-rose-400",
    ring: "ring-rose-500/20",
    border: "border-rose-100",
  },
};

export function StatCard({
  label,
  value,
  detail,
  tone = "green",
  icon,
  trend,
  className = "",
}: StatCardProps) {
  const styles = toneStyles[tone];

  return (
    <article
      className={`rounded-xl border border-zinc-200/80 bg-white p-5 shadow-xs transition-all hover:shadow-sm ${className}`}
    >
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
          {label}
        </p>
        {icon && (
          <div
            aria-hidden="true"
            className={`flex h-9 w-9 items-center justify-center rounded-lg border ${styles.border} ${styles.bg} ${styles.text}`}
          >
            {icon}
          </div>
        )}
      </div>
      <div className="mt-3 flex items-baseline gap-2">
        <output
          aria-label={label}
          className="text-2xl font-bold tracking-tight text-zinc-900"
        >
          {value}
        </output>
        {trend && (
          <span
            className={`text-xs font-semibold ${
              trend.isPositive ? "text-emerald-600" : "text-rose-600"
            }`}
          >
            {trend.value}
          </span>
        )}
      </div>
      <p className="mt-1 text-xs text-zinc-500">{detail}</p>
    </article>
  );
}
