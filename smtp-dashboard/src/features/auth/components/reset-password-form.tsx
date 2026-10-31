"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import {
  getAuthCapabilities,
  resetPassword,
} from "../services/password-auth-api";

export function ResetPasswordForm({ token }: { token: string }) {
  const router = useRouter();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [password, setPassword] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [complete, setComplete] = useState(false);

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
    if (password !== confirmation) {
      setError("The passwords do not match.");
      return;
    }
    if (!token) {
      setError("This reset link is incomplete. Request a new link.");
      return;
    }
    setBusy(true);
    try {
      await resetPassword(token, password);
      setComplete(true);
      router.replace("/reset-password");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Password reset failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="reset-password-heading" className="space-y-4">
      <div>
        <h1 className="text-xl font-semibold text-zinc-900" id="reset-password-heading">
          Choose a new password
        </h1>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Reset links expire after 30 minutes and can only be used once.
        </p>
      </div>
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      {complete && (
        <p className="notice notice-success" role="status">
          Your password has been changed and existing sessions were signed out.
        </p>
      )}
      {!complete && (enabled === null ? (
        <p aria-live="polite" className="text-sm text-zinc-600" role="status">Checking recovery configuration…</p>
      ) : enabled ? (
        <form className="space-y-3" onSubmit={(event) => void submit(event)}>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            New password
            <input
              autoComplete="new-password"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              maxLength={1024}
              minLength={12}
              onChange={(event) => setPassword(event.target.value)}
              required
              type="password"
              value={password}
            />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Confirm new password
            <input
              autoComplete="new-password"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              maxLength={1024}
              minLength={12}
              onChange={(event) => setConfirmation(event.target.value)}
              required
              type="password"
              value={confirmation}
            />
          </label>
          <button
            className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800 disabled:opacity-60"
            disabled={busy}
            type="submit"
          >
            {busy ? "Updating password…" : "Update password"}
          </button>
        </form>
      ) : (
        <p className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm leading-5 text-amber-900">
          Password recovery is not configured for this deployment.
        </p>
      ))}
      <Link className="text-sm font-medium text-emerald-800 underline underline-offset-4" href="/login">
        Return to sign in
      </Link>
    </section>
  );
}
