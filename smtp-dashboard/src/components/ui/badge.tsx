import type { HTMLAttributes, ReactNode } from "react";

export type BadgeVariant =
  | "success"
  | "warning"
  | "error"
  | "info"
  | "neutral"
  | "pending";

interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: BadgeVariant;
  withDot?: boolean;
  children: ReactNode;
  size?: "sm" | "md";
}

const variantStyles: Record<
  BadgeVariant,
  { bg: string; text: string; dot: string }
> = {
  success: {
    bg: "bg-emerald-50 text-emerald-800 border-emerald-200 dark:bg-emerald-950/40 dark:text-emerald-300 dark:border-emerald-800",
    text: "text-emerald-700 dark:text-emerald-300",
    dot: "bg-emerald-500",
  },
  warning: {
    bg: "bg-amber-50 text-amber-800 border-amber-200 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-800",
    text: "text-amber-700 dark:text-amber-300",
    dot: "bg-amber-500",
  },
  error: {
    bg: "bg-rose-50 text-rose-800 border-rose-200 dark:bg-rose-950/40 dark:text-rose-300 dark:border-rose-800",
    text: "text-rose-700 dark:text-rose-300",
    dot: "bg-rose-500",
  },
  info: {
    bg: "bg-sky-50 text-sky-800 border-sky-200 dark:bg-sky-950/40 dark:text-sky-300 dark:border-sky-800",
    text: "text-sky-700 dark:text-sky-300",
    dot: "bg-sky-500",
  },
  pending: {
    bg: "bg-amber-50 text-amber-800 border-amber-200 dark:bg-amber-950/40 dark:text-amber-300 dark:border-amber-800",
    text: "text-amber-700 dark:text-amber-300",
    dot: "bg-amber-500 animate-pulse",
  },
  neutral: {
    bg: "bg-zinc-100 text-zinc-700 border-zinc-200 dark:bg-zinc-800 dark:text-zinc-300 dark:border-zinc-700",
    text: "text-zinc-600 dark:text-zinc-400",
    dot: "bg-zinc-400",
  },
};

export function Badge({
  variant = "neutral",
  withDot = false,
  size = "md",
  children,
  className = "",
  ...props
}: BadgeProps) {
  const styles = variantStyles[variant];
  const sizeClasses =
    size === "sm" ? "px-2 py-0.5 text-[10px]" : "px-2.5 py-1 text-xs";

  return (
    <span
      className={`inline-flex items-center gap-1.5 font-medium rounded-full border ${styles.bg} ${sizeClasses} ${className}`}
      {...props}
    >
      {withDot && (
        <span
          className={`h-1.5 w-1.5 rounded-full ${styles.dot}`}
          aria-hidden="true"
        />
      )}
      {children}
    </span>
  );
}
