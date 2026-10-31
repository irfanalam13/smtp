"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { ApiError } from "@/lib/api/json-client";
import {
  getAuthCapabilities,
  resendVerification,
  signUp,
} from "../services/password-auth-api";

export function SignupForm() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [organization, setOrganization] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [canResend, setCanResend] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    void getAuthCapabilities(controller.signal)
      .then((capabilities) => {
        if (!controller.signal.aborted) {
          setEnabled(capabilities.signup_enabled);
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) {
          setEnabled(false);
          setError("Registration configuration could not be loaded. Try again later.");
        }
      });
    return () => controller.abort();
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setMessage("");
    setBusy(true);
    try {
      const response = await signUp({
        email,
        password,
        organization_name: organization,
      });
      setMessage(response.message);
      setCanResend(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Registration failed.");
      if (reason instanceof ApiError && reason.code === "email_unavailable") {
        setCanResend(true);
      }
    } finally {
      setBusy(false);
    }
  }

  async function resend() {
    setError("");
    setMessage("");
    setBusy(true);
    try {
      await resendVerification(email);
      setMessage("If verification is still needed, a new link has been sent.");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The request failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="signup-heading" className="space-y-4">
      <div>
        <p className="text-xs font-semibold tracking-[0.18em] text-emerald-700">SMTP PLATFORM</p>
        <h1 className="mt-2 text-xl font-semibold text-zinc-900" id="signup-heading">
          Create your account
        </h1>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Create a workspace. Email verification is required before sign-in.
        </p>
      </div>
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      {message && <p className="notice notice-success" role="status">{message}</p>}
      {enabled === null ? (
        <p aria-live="polite" className="text-sm text-zinc-600" role="status">Checking registration…</p>
      ) : enabled ? message ? (
        <div className="space-y-3">
          <button
            className="button button-quiet"
            disabled={busy || !email}
            onClick={() => void resend()}
            type="button"
          >
            {busy ? "Sending…" : "Resend verification email"}
          </button>
          <p className="text-sm text-zinc-600">
            Already verified? <Link className="font-medium text-emerald-800 underline" href="/login">Sign in</Link>
          </p>
        </div>
      ) : (
        <div className="space-y-3">
        <form className="space-y-3" onSubmit={(event) => void submit(event)}>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Work email
            <input
              autoComplete="email"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              onChange={(event) => setEmail(event.target.value)}
              required
              type="email"
              value={email}
            />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Organization name
            <input
              autoComplete="organization"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              maxLength={160}
              onChange={(event) => setOrganization(event.target.value)}
              required
              value={organization}
            />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Password
            <input
              autoComplete="new-password"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              minLength={12}
              onChange={(event) => setPassword(event.target.value)}
              required
              type="password"
              value={password}
            />
            <span className="block text-xs font-normal text-zinc-500">At least 12 characters.</span>
          </label>
          <button
            className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800 disabled:opacity-60"
            disabled={busy}
            type="submit"
          >
            {busy ? "Creating account…" : "Create account"}
          </button>
        </form>
        {canResend && (
          <button
            className="text-sm font-medium text-emerald-800 underline underline-offset-4 disabled:opacity-60"
            disabled={busy || !email}
            onClick={() => void resend()}
            type="button"
          >
            {busy ? "Sending…" : "Resend verification email"}
          </button>
        )}
        </div>
      ) : (
        <p className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm leading-5 text-amber-900">
          Registration is not configured for this deployment. Contact the platform administrator.
        </p>
      )}
      <p className="text-sm text-zinc-600">
        Already have an account? <Link className="font-medium text-emerald-800 underline" href="/login">Sign in</Link>
      </p>
    </section>
  );
}
