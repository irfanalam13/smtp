import type { ReactNode } from "react";
import { AlertCircleIcon, CheckCircleIcon, InfoIcon, XIcon } from "../ui/icons";

export type AlertTone = "info" | "success" | "warning" | "error";

interface AlertProps {
  tone?: AlertTone;
  title?: string;
  message?: ReactNode;
  children?: ReactNode;
  onDismiss?: () => void;
  className?: string;
}

const toneMap: Record<
  AlertTone,
  {
    container: string;
    icon: typeof InfoIcon;
    iconColor: string;
    titleColor: string;
  }
> = {
  info: {
    container: "bg-sky-50/80 border-sky-200 text-sky-900",
    icon: InfoIcon,
    iconColor: "text-sky-600",
    titleColor: "text-sky-950",
  },
  success: {
    container: "bg-emerald-50/80 border-emerald-200 text-emerald-900",
    icon: CheckCircleIcon,
    iconColor: "text-emerald-600",
    titleColor: "text-emerald-950",
  },
  warning: {
    container: "bg-amber-50/80 border-amber-200 text-amber-900",
    icon: AlertCircleIcon,
    iconColor: "text-amber-600",
    titleColor: "text-amber-950",
  },
  error: {
    container: "bg-rose-50/80 border-rose-200 text-rose-900",
    icon: AlertCircleIcon,
    iconColor: "text-rose-600",
    titleColor: "text-rose-950",
  },
};

export function Alert({
  tone = "info",
  title,
  message,
  children,
  onDismiss,
  className = "",
}: AlertProps) {
  const current = toneMap[tone];
  const Icon = current.icon;

  return (
    <div
      role={tone === "error" ? "alert" : "status"}
      aria-live="polite"
      className={`relative flex items-start gap-3 rounded-xl border p-4 text-sm transition-all shadow-xs ${current.container} ${className}`}
    >
      <div className={`shrink-0 pt-0.5 ${current.iconColor}`}>
        <Icon className="h-5 w-5" />
      </div>
      <div className="flex-1 space-y-1">
        {title && (
          <h4 className={`font-semibold ${current.titleColor}`}>{title}</h4>
        )}
        <div className="text-xs leading-relaxed text-zinc-700">
          {message || children}
        </div>
      </div>
      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss alert"
          className="shrink-0 -mr-1 -mt-1 p-1 rounded-lg text-zinc-400 hover:text-zinc-700 hover:bg-black/5 transition-colors"
        >
          <XIcon className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}
