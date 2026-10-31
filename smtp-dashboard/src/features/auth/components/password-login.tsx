"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { ApiError } from "@/lib/api/json-client";
import {
  getAuthCapabilities,
  passwordLogin,
} from "../services/password-auth-api";

export function PasswordLogin() {
  const router = useRouter();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [tenantID, setTenantID] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const controller = new AbortController();
    void getAuthCapabilities(controller.signal)
      .then((capabilities) => {
        if (!controller.signal.aborted) {
          setEnabled(capabilities.password_login_enabled);
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) {
          setEnabled(false);
          setError("Sign-in configuration could not be loaded. Try again later.");
        }
      });
    return () => controller.abort();
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      await passwordLogin({
        email,
        password,
        ...(tenantID.trim() ? { tenant_id: tenantID.trim() } : {}),
      });
      router.replace("/home");
      router.refresh();
    } catch (reason) {
      const code = reason instanceof ApiError ? reason.code : null;
      setError(
        code === "tenant_selection_required"
          ? "This account belongs to multiple organizations. Enter the organization ID and try again."
          : code === "email_not_verified"
            ? "Verify your email address before signing in. Use the resend-verification option on the signup page if needed."
            : reason instanceof Error
              ? reason.message
              : "Sign-in failed. Please try again.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="password-login-heading" className="space-y-4">
      <div>
        <h1 className="text-xl font-semibold text-zinc-900" id="password-login-heading">
          Sign in with email
        </h1>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Use your verified SMTP Platform account.
        </p>
      </div>
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      {enabled === null ? (
        <p aria-live="polite" className="text-sm text-zinc-600" role="status">
          Checking account sign-in…
        </p>
      ) : enabled ? (
        <form className="space-y-3" onSubmit={(event) => void submit(event)}>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Email address
            <input
              autoComplete="email"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm focus-visible:outline-2 focus-visible:outline-emerald-700"
              onChange={(event) => setEmail(event.target.value)}
              required
              type="email"
              value={email}
            />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Password
            <input
              autoComplete="current-password"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm focus-visible:outline-2 focus-visible:outline-emerald-700"
              onChange={(event) => setPassword(event.target.value)}
              required
              type="password"
              value={password}
            />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Organization ID <span className="font-normal text-zinc-500">(optional)</span>
            <input
              autoComplete="off"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm focus-visible:outline-2 focus-visible:outline-emerald-700"
              onChange={(event) => setTenantID(event.target.value)}
              placeholder="Required if your account has multiple organizations"
              value={tenantID}
            />
          </label>
          <button
            className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={busy}
            type="submit"
          >
            {busy ? "Signing in…" : "Sign in"}
          </button>
        </form>
      ) : (
        <p className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm leading-5 text-amber-900">
          Password sign-in is not configured for this deployment.
        </p>
      )}
      <div className="flex flex-wrap justify-between gap-2 text-sm">
        <Link className="font-medium text-emerald-800 underline underline-offset-4" href="/forgot-password">
          Forgot password?
        </Link>
        <Link className="font-medium text-emerald-800 underline underline-offset-4" href="/signup">
          Create an account
        </Link>
      </div>
    </section>
  );
}
