"use client";

import { useState } from "react";
import { CheckIcon, CopyIcon } from "./icons";

interface CodeBlockProps {
  code: string;
  label?: string;
  masked?: boolean;
  className?: string;
}

export function CodeBlock({
  code,
  label,
  masked = false,
  className = "",
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [revealed, setRevealed] = useState(!masked);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback
    }
  }

  const displayText = revealed
    ? code
    : "•".repeat(Math.min(code.length || 24, 32));

  return (
    <div
      className={`rounded-lg border border-zinc-200 bg-zinc-900 text-zinc-100 overflow-hidden font-mono text-xs ${className}`}
    >
      {label && (
        <div className="flex items-center justify-between border-b border-zinc-800 bg-zinc-950/60 px-3 py-1.5 text-[11px] text-zinc-400 font-sans">
          <span>{label}</span>
          {masked && (
            <button
              type="button"
              onClick={() => setRevealed(!revealed)}
              className="text-xs text-zinc-400 hover:text-zinc-200 transition-colors"
            >
              {revealed ? "Hide" : "Show"}
            </button>
          )}
        </div>
      )}
      <div className="flex items-center justify-between p-3 gap-3">
        <code className="overflow-x-auto break-all font-mono selection:bg-emerald-800 selection:text-white">
          {displayText}
        </code>
        <button
          type="button"
          onClick={handleCopy}
          aria-label={copied ? "Copied" : "Copy to clipboard"}
          title={copied ? "Copied" : "Copy to clipboard"}
          className={`shrink-0 flex items-center gap-1.5 px-2.5 py-1.5 rounded-md text-xs font-sans font-medium transition-all ${
            copied
              ? "bg-emerald-600/20 text-emerald-400 border border-emerald-500/30"
              : "bg-zinc-800 hover:bg-zinc-700 text-zinc-300 border border-zinc-700 hover:text-white"
          }`}
        >
          {copied ? (
            <>
              <CheckIcon className="h-3.5 w-3.5" />
              <span>Copied</span>
            </>
          ) : (
            <>
              <CopyIcon className="h-3.5 w-3.5" />
              <span>Copy</span>
            </>
          )}
        </button>
      </div>
    </div>
  );
}
