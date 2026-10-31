import type { RefObject } from "react";
import { MenuIcon, ShieldIcon } from "../ui/icons";
import type {
  Principal,
  Tenant,
} from "@/features/control-plane/types/control-plane";

interface TopbarProps {
  title: string;
  mobileOpen: boolean;
  onOpenMobile: () => void;
  menuButtonRef: RefObject<HTMLButtonElement | null>;
  connected: boolean;
  loading: boolean;
  principal: Principal | null;
  tenant: Tenant | null;
  onCredentialsClick: () => void;
}

export function Topbar({
  title,
  mobileOpen,
  onOpenMobile,
  menuButtonRef,
  connected,
  loading,
  principal,
  tenant,
  onCredentialsClick,
}: TopbarProps) {
  return (
    <header className="sticky top-0 z-10 flex h-16 w-full items-center justify-between border-b border-zinc-200/80 bg-white/95 px-4 sm:px-6 backdrop-blur-md">
      <div className="flex items-center gap-3">
        <button
          ref={menuButtonRef}
          type="button"
          aria-controls="dashboard-sidebar"
          aria-expanded={mobileOpen}
          aria-label="Open dashboard navigation"
          onClick={onOpenMobile}
          className="md:hidden flex h-9 w-9 items-center justify-center rounded-lg border border-zinc-200 text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 transition-colors"
        >
          <MenuIcon className="h-5 w-5" />
        </button>

        <nav
          aria-label="Breadcrumb"
          className="flex items-center gap-2 text-xs"
        >
          <span className="font-medium text-zinc-400">Workspace</span>
          <span className="text-zinc-300">/</span>
          <strong className="font-semibold text-zinc-900">{title}</strong>
        </nav>
      </div>

      <div className="flex items-center gap-3">
        {/* Backend health pill */}
        <div
          role="status"
          aria-live="polite"
          className="hidden sm:flex items-center gap-2 rounded-full border border-zinc-200/80 bg-zinc-50/80 px-3 py-1 text-xs"
        >
          <span
            className={`h-2 w-2 rounded-full ${
              connected
                ? "bg-emerald-500 ring-2 ring-emerald-500/20 animate-pulse"
                : loading
                  ? "bg-amber-400 animate-ping"
                  : "bg-rose-500"
            }`}
            aria-hidden="true"
          />
          <span className="text-[11px] font-medium text-zinc-600">
            {connected
              ? "API Connected"
              : loading
                ? "Connecting..."
                : "API Offline"}
          </span>
        </div>

        {/* Credentials / identity button */}
        <button
          type="button"
          onClick={onCredentialsClick}
          title="Account credentials"
          className="flex items-center gap-2 rounded-lg border border-zinc-200/90 bg-white px-2.5 py-1.5 text-xs text-zinc-700 hover:bg-zinc-50 hover:border-zinc-300 transition-colors"
        >
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-emerald-100 text-emerald-800 text-[11px] font-bold">
            {(principal?.subject?.[0] ?? "U").toUpperCase()}
          </div>
          <div className="text-left hidden lg:block">
            <div className="font-semibold leading-tight text-zinc-900 max-w-[120px] truncate">
              {principal?.subject || "Local Dev"}
            </div>
            <div className="text-[10px] text-zinc-400 leading-tight">
              {tenant?.name || "Default Tenant"}
            </div>
          </div>
          <ShieldIcon className="h-3.5 w-3.5 text-zinc-400 ml-0.5" />
        </button>
      </div>
    </header>
  );
}
