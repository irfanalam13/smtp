"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import {
  getAuthCapabilities,
  requestPasswordReset,
} from "../services/password-auth-api";

export function ForgotPasswordForm() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [submitted, setSubmitted] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    void getAuthCapabilities(controller.signal)
      .then((capabilities) => {
        if (!controller.signal.aborted) {
          setEnabled(capabilities.password_reset_enabled);
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) {
          setEnabled(false);
          setError("Password recovery configuration could not be loaded.");
        }
      });
    return () => controller.abort();
  }, []);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      await requestPasswordReset(email);
      setSubmitted(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The request failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="forgot-password-heading" className="space-y-4">
      <div>
        <h1 className="text-xl font-semibold text-zinc-900" id="forgot-password-heading">
          Reset your password
        </h1>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Enter your account email and we&apos;ll send a recovery link if an account can be recovered.
        </p>
      </div>
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      {submitted && (
        <p className="notice notice-success" role="status">
          If an account can be recovered, password-reset instructions have been sent.
        </p>
      )}
      {enabled === null ? (
        <p aria-live="polite" className="text-sm text-zinc-600" role="status">Checking recovery configuration…</p>
      ) : enabled ? (
        <form className="space-y-3" onSubmit={(event) => void submit(event)}>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Email address
            <input
              autoComplete="email"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              disabled={submitted}
              onChange={(event) => setEmail(event.target.value)}
              required
              type="email"
              value={email}
            />
          </label>
          <button
            className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800 disabled:opacity-60"
            disabled={busy || submitted}
            type="submit"
          >
            {busy ? "Sending…" : "Send recovery link"}
          </button>
        </form>
      ) : (
        <p className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm leading-5 text-amber-900">
          Password recovery is not configured for this deployment.
        </p>
      )}
      <Link className="text-sm font-medium text-emerald-800 underline underline-offset-4" href="/login">
        Return to sign in
      </Link>
    </section>
  );
}
