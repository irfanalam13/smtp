"use client";

import {
  useCallback,
  useEffect,
  useState,
  useSyncExternalStore,
} from "react";
import { requestJson } from "@/lib/api/json-client";

type ProviderConfiguration = {
  oidc_enabled: boolean;
  google_enabled?: boolean;
  github_enabled?: boolean;
};

type ProviderState =
  | { kind: "loading" }
  | {
      kind: "ready";
      enabled: boolean;
      googleEnabled: boolean;
      githubEnabled: boolean;
    }
  | { kind: "error" };

const publicContext = { token: "", tenantId: "" };

const callbackErrors: Record<string, string> = {
  oidc_failed:
    "Your identity provider could not complete sign-in. Please try again.",
  tenant_selection_required:
    "This account belongs to more than one organization. Enter the organization ID and try again.",
  session_expired:
    "Your session ended. Sign in again to continue.",
};

function subscribeToLocationChange(callback: () => void) {
  window.addEventListener("popstate", callback);
  return () => window.removeEventListener("popstate", callback);
}

function getCallbackError() {
  const code = new URLSearchParams(window.location.search).get("error");
  return code ? (callbackErrors[code] ?? callbackErrors.oidc_failed) : "";
}

export function OIDCSignIn() {
  const [provider, setProvider] = useState<ProviderState>({ kind: "loading" });
  const [tenantID, setTenantID] = useState("");
  const authError = useSyncExternalStore(
    subscribeToLocationChange,
    getCallbackError,
    () => "",
  );

  const loadProvider = useCallback(async (
    signal?: AbortSignal,
  ): Promise<ProviderState | null> => {
    try {
      const configuration = await requestJson<ProviderConfiguration>(
        "/v1/auth/providers",
        publicContext,
        { signal },
      );
      return signal?.aborted
        ? null
        : {
            kind: "ready",
            enabled: configuration.oidc_enabled,
            googleEnabled: Boolean(configuration.google_enabled),
            githubEnabled: Boolean(configuration.github_enabled),
          };
    } catch {
      return signal?.aborted ? null : { kind: "error" };
    }
  }, []);

  useEffect(() => {
    let mounted = true;
    void loadProvider().then((result) => {
      if (mounted && result) {
        setProvider(result);
      }
    });
    return () => {
      mounted = false;
    };
  }, [loadProvider]);

  const query = tenantID.trim()
    ? `?tenant_id=${encodeURIComponent(tenantID.trim())}`
    : "";
  const signInURL = `/api/backend/v1/auth/oidc/start${query}`;
  const tenantIDValid =
    tenantID.trim() === "" ||
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(
      tenantID.trim(),
    );

  return (
    <section
      aria-labelledby="enterprise-signin-heading"
      className="w-full space-y-4"
    >
      <div>
        <p className="text-xs font-semibold tracking-[0.18em] text-emerald-700">
          SMTP PLATFORM
        </p>
        <h2
          className="mt-2 text-xl font-semibold text-zinc-900"
          id="enterprise-signin-heading"
        >
          Organization sign-in
        </h2>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Continue through your organization&apos;s configured OpenID Connect
          identity provider.
        </p>
      </div>

      {authError && (
        <p className="notice notice-error" role="alert">
          {authError}
        </p>
      )}

      {provider.kind === "ready" && (provider.googleEnabled || provider.githubEnabled) && (
        <div className="space-y-2.5">
          {provider.googleEnabled && (
            <a
              className="inline-flex min-h-11 w-full items-center justify-center gap-2.5 rounded-lg border border-zinc-300 bg-white px-4 text-sm font-semibold text-zinc-700 shadow-xs transition hover:bg-zinc-50 hover:text-zinc-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-700"
              href={`/api/backend/v1/auth/oauth/google/start${query}`}
            >
              <svg aria-hidden="true" className="h-4 w-4 shrink-0" viewBox="0 0 24 24">
                <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" />
                <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" />
                <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z" />
                <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z" />
              </svg>
              Continue with Google
            </a>
          )}
          {provider.githubEnabled && (
            <a
              className="inline-flex min-h-11 w-full items-center justify-center gap-2.5 rounded-lg border border-zinc-800 bg-zinc-900 px-4 text-sm font-semibold text-white shadow-xs transition hover:bg-zinc-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-zinc-900"
              href={`/api/backend/v1/auth/oauth/github/start${query}`}
            >
              <svg aria-hidden="true" className="h-4 w-4 shrink-0 fill-current" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
              </svg>
              Continue with GitHub
            </a>
          )}
          <div aria-hidden="true" className="flex items-center gap-3 py-1">
            <span className="h-px flex-1 bg-zinc-200" />
            <span className="text-[11px] font-medium uppercase tracking-wider text-zinc-400">or enterprise sso</span>
            <span className="h-px flex-1 bg-zinc-200" />
          </div>
        </div>
      )}

      <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
        Organization ID
        <input
          aria-invalid={!tenantIDValid}
          autoComplete="off"
          className="h-10 w-full rounded-lg border border-zinc-300 bg-white px-3 text-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-700"
          onChange={(event) => setTenantID(event.target.value)}
          placeholder="Optional; required for multi-organization accounts"
          value={tenantID}
        />
      </label>
      {!tenantIDValid && (
        <p className="text-xs text-rose-700" role="alert">
          Enter a valid organization UUID or leave this field empty.
        </p>
      )}

      {provider.kind === "loading" && (
        <p aria-live="polite" className="text-sm text-zinc-600" role="status">
          Checking available sign-in methods…
        </p>
      )}
      {provider.kind === "error" && (
        <div className="space-y-2" role="alert">
          <p className="text-sm text-rose-700">
            Sign-in configuration could not be loaded.
          </p>
          <button
            className="text-sm font-medium text-emerald-800 underline underline-offset-4"
            onClick={() => {
              setProvider({ kind: "loading" });
              void loadProvider().then((result) => {
                if (result) {
                  setProvider(result);
                }
              });
            }}
            type="button"
          >
            Try again
          </button>
        </div>
      )}
      {provider.kind === "ready" && provider.enabled && tenantIDValid && (
        <a
          className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white transition-colors hover:bg-emerald-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-700"
          href={signInURL}
        >
          Continue with organization SSO
        </a>
      )}
      {provider.kind === "ready" && !provider.enabled && (
        <p className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm leading-5 text-amber-900">
          Enterprise sign-in is not configured for this deployment. Contact
          your platform administrator; no sign-in provider is being simulated.
        </p>
      )}
      <p className="text-xs leading-5 text-zinc-500">
        Your organization account must already be linked to an active SMTP
        Platform membership. Email-address matching does not link accounts.
      </p>
    </section>
  );
}
