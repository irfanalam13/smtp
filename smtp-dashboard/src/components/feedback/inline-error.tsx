import { AlertCircleIcon, RefreshCwIcon } from "../ui/icons";

interface InlineErrorProps {
  message: string;
  onRetry?: () => void;
  className?: string;
}

export function InlineError({
  message,
  onRetry,
  className = "",
}: InlineErrorProps) {
  return (
    <div
      role="alert"
      className={`flex items-center justify-between gap-3 rounded-lg border border-rose-200 bg-rose-50/70 p-3.5 text-xs text-rose-800 shadow-xs ${className}`}
    >
      <div className="flex items-center gap-2">
        <AlertCircleIcon className="h-4 w-4 shrink-0 text-rose-600" />
        <span>{message}</span>
      </div>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          className="shrink-0 inline-flex items-center gap-1 font-semibold text-rose-700 hover:text-rose-900 underline underline-offset-2 hover:bg-rose-100/50 px-2 py-1 rounded transition-colors"
        >
          <RefreshCwIcon className="h-3 w-3" />
          <span>Try again</span>
        </button>
      )}
    </div>
  );
}
