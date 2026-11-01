"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { AlertCircleIcon, CheckCircleIcon, InfoIcon, XIcon } from "../ui/icons";

export type ToastTone = "info" | "success" | "warning" | "error";

type ToastInput = {
  title?: string;
  message: string;
  tone?: ToastTone;
  duration?: number;
};

type Toast = ToastInput & { id: number };

type ToastContextValue = {
  toast: (input: ToastInput) => void;
  dismissToast: (id: number) => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);
let nextToastID = 1;

const toneMap: Record<
  ToastTone,
  { container: string; icon: typeof InfoIcon; iconColor: string }
> = {
  info: {
    container: "border-sky-200 bg-sky-50 text-sky-950",
    icon: InfoIcon,
    iconColor: "text-sky-600",
  },
  success: {
    container: "border-emerald-200 bg-emerald-50 text-emerald-950",
    icon: CheckCircleIcon,
    iconColor: "text-emerald-600",
  },
  warning: {
    container: "border-amber-200 bg-amber-50 text-amber-950",
    icon: AlertCircleIcon,
    iconColor: "text-amber-600",
  },
  error: {
    container: "border-rose-200 bg-rose-50 text-rose-950",
    icon: AlertCircleIcon,
    iconColor: "text-rose-600",
  },
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((item) => item.id !== id));
  }, []);

  const toast = useCallback((input: ToastInput) => {
    const item = { ...input, id: nextToastID++ };
    setToasts((current) => [...current.slice(-3), item]);
  }, []);

  const value = useMemo(() => ({ toast, dismissToast }), [dismissToast, toast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div
        aria-label="Notifications"
        className="fixed inset-x-4 top-4 z-50 flex max-w-md flex-col gap-3 sm:left-auto sm:right-6 sm:max-w-sm"
        role="region"
      >
        {toasts.map((item) => (
          <ToastItem item={item} dismissToast={dismissToast} key={item.id} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function ToastItem({
  item,
  dismissToast,
}: {
  item: Toast;
  dismissToast: (id: number) => void;
}) {
  useEffect(() => {
    const timeout = window.setTimeout(
      () => dismissToast(item.id),
      item.duration ?? 3000,
    );
    return () => window.clearTimeout(timeout);
  }, [dismissToast, item.duration, item.id]);

  const tone = toneMap[item.tone ?? "info"];
  const Icon = tone.icon;

  return (
    <div
      aria-live={item.tone === "error" ? "assertive" : "polite"}
      className={`flex items-start gap-3 rounded-xl border p-4 text-sm shadow-lg ${tone.container}`}
    >
      <Icon className={`mt-0.5 h-5 w-5 shrink-0 ${tone.iconColor}`} />
      <div className="min-w-0 flex-1">
        {item.title && <p className="font-semibold">{item.title}</p>}
        <p className="mt-0.5 leading-5">{item.message}</p>
      </div>
      <button
        aria-label="Dismiss notification"
        className="-mr-1 -mt-1 shrink-0 rounded-lg p-1 text-zinc-500 hover:bg-black/5 hover:text-zinc-900"
        onClick={() => dismissToast(item.id)}
        type="button"
      >
        <XIcon className="h-4 w-4" />
      </button>
    </div>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used inside ToastProvider");
  }
  return context;
}
