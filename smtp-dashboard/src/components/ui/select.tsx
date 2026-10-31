import { forwardRef } from "react";
import type { SelectHTMLAttributes } from "react";
import { ChevronDownIcon } from "./icons";

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  error?: string;
  hint?: string;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  (
    { id, label, error, hint, required, children, className = "", ...props },
    ref,
  ) => {
    return (
      <div className="w-full space-y-1.5">
        {label && (
          <label
            htmlFor={id}
            className="block text-xs font-semibold text-zinc-700 select-none"
          >
            {label} {required && <span className="text-rose-500">*</span>}
          </label>
        )}
        <div className="relative">
          <select
            id={id}
            ref={ref}
            required={required}
            aria-invalid={Boolean(error)}
            aria-describedby={
              error ? `${id}-error` : hint ? `${id}-hint` : undefined
            }
            className={`w-full appearance-none rounded-lg border bg-white px-3 py-2 pr-9 text-sm text-zinc-900 transition-all outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 disabled:opacity-60 disabled:bg-zinc-50 disabled:cursor-not-allowed ${
              error
                ? "border-rose-300 focus:border-rose-500 focus:ring-rose-500/20"
                : "border-zinc-200"
            } ${className}`}
            {...props}
          >
            {children}
          </select>
          <div className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400">
            <ChevronDownIcon className="h-4 w-4" />
          </div>
        </div>
        {error ? (
          <p id={`${id}-error`} role="alert" className="text-xs text-rose-600">
            {error}
          </p>
        ) : hint ? (
          <p id={`${id}-hint`} className="text-xs text-zinc-500">
            {hint}
          </p>
        ) : null}
      </div>
    );
  },
);

Select.displayName = "Select";
