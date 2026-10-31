import { AlertCircleIcon, RefreshCwIcon } from "@/components/ui/icons";

export function InlineError({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="inline-error" role="alert">
      <div className="flex items-center gap-2">
        <AlertCircleIcon className="h-4 w-4 shrink-0 text-rose-600" aria-hidden="true" />
        <p>{message}</p>
      </div>
      {onRetry && (
        <button className="text-button inline-flex items-center gap-1.5" onClick={onRetry} type="button">
          <RefreshCwIcon className="h-3 w-3" aria-hidden="true" />
          <span>Try again</span>
        </button>
      )}
    </div>
  );
}

export function EmptyState({
  message,
  loading = false,
}: {
  message: string;
  loading?: boolean;
}) {
  return (
    <div
      className="empty-state"
      role={loading ? "status" : undefined}
      aria-live="polite"
      aria-busy={loading}
    >
      {loading ? (
        <span className="spinner" aria-hidden="true">
          <svg
            className="animate-spin h-5 w-5 text-emerald-600"
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
        </span>
      ) : null}
      <p>{message}</p>
    </div>
  );
}
