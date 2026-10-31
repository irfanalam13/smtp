"use client";

import Link from "next/link";
import { useState } from "react";
import { verifyEmail } from "../services/password-auth-api";

export function VerifyEmailForm({ token }: { token: string }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [verified, setVerified] = useState(false);

  async function verify() {
    setError("");
    if (!token) {
      setError("This verification link is incomplete. Request another verification email.");
      return;
    }
    setBusy(true);
    try {
      await verifyEmail(token);
      setVerified(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Email verification failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section aria-labelledby="verify-email-heading" className="space-y-4">
      <div>
        <h1 className="text-xl font-semibold text-zinc-900" id="verify-email-heading">
          Verify your email
        </h1>
        <p className="mt-1 text-sm leading-6 text-zinc-600">
          Confirm your email address to activate password sign-in.
        </p>
      </div>
      {error && <p className="notice notice-error" role="alert">{error}</p>}
      {verified ? (
        <div className="space-y-3">
          <p className="notice notice-success" role="status">Your email is verified. You can now sign in.</p>
          <Link className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white" href="/login">
            Continue to sign in
          </Link>
        </div>
      ) : (
        <button
          className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800 disabled:opacity-60"
          disabled={busy}
          onClick={() => void verify()}
          type="button"
        >
          {busy ? "Verifying…" : "Verify email address"}
        </button>
      )}
    </section>
  );
}
