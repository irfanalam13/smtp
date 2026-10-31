import type { ReactNode } from "react";
import { InboxIcon } from "../ui/icons";

interface EmptyStateProps {
  title?: string;
  message: string;
  icon?: ReactNode;
  loading?: boolean;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({
  title,
  message,
  icon,
  loading = false,
  action,
  className = "",
}: EmptyStateProps) {
  return (
    <div
      role={loading ? "status" : undefined}
      aria-live="polite"
      aria-busy={loading}
      className={`flex flex-col items-center justify-center p-8 text-center rounded-xl border border-dashed border-zinc-200 bg-zinc-50/50 ${className}`}
    >
      {loading ? (
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-emerald-50 text-emerald-600 mb-3 animate-spin">
          <svg
            className="h-6 w-6"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"
            />
          </svg>
        </div>
      ) : (
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-zinc-100 text-zinc-400 mb-3">
          {icon || <InboxIcon className="h-6 w-6" />}
        </div>
      )}
      {title && (
        <h4 className="text-sm font-semibold text-zinc-900">{title}</h4>
      )}
      <p className="mt-1 text-xs text-zinc-500 max-w-sm">{message}</p>
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
