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
};

type ProviderState =
  | { kind: "loading" }
  | { kind: "ready"; enabled: boolean }
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
        : { kind: "ready", enabled: configuration.oidc_enabled };
    } catch {
      return signal?.aborted ? null : { kind: "error" };
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    void loadProvider(controller.signal).then((result) => {
      if (result) {
        setProvider(result);
      }
    });
    return () => controller.abort();
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
