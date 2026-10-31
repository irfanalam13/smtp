import { forwardRef } from "react";
import type { InputHTMLAttributes, ReactNode } from "react";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
  hint?: string;
  leftIcon?: ReactNode;
  rightElement?: ReactNode;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    {
      id,
      label,
      error,
      hint,
      leftIcon,
      rightElement,
      className = "",
      required,
      ...props
    },
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
        <div className="relative flex items-center">
          {leftIcon && (
            <div className="absolute left-3 pointer-events-none text-zinc-400">
              {leftIcon}
            </div>
          )}
          <input
            id={id}
            ref={ref}
            required={required}
            aria-invalid={Boolean(error)}
            aria-describedby={
              error ? `${id}-error` : hint ? `${id}-hint` : undefined
            }
            className={`w-full rounded-lg border bg-white px-3 py-2 text-sm text-zinc-900 placeholder:text-zinc-400 transition-all outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 disabled:opacity-60 disabled:bg-zinc-50 disabled:cursor-not-allowed ${
              leftIcon ? "pl-9" : ""
            } ${rightElement ? "pr-10" : ""} ${
              error
                ? "border-rose-300 focus:border-rose-500 focus:ring-rose-500/20"
                : "border-zinc-200"
            } ${className}`}
            {...props}
          />
          {rightElement && (
            <div className="absolute right-2.5 flex items-center">
              {rightElement}
            </div>
          )}
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

Input.displayName = "Input";
