"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "@/components/feedback/toast";
import { ApiError } from "@/lib/api/json-client";
import {
  getAuthCapabilities,
  resendVerification,
  signUp,
} from "../services/password-auth-api";

export function SignupForm() {
  const { toast } = useToast();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [termsAccepted, setTermsAccepted] = useState(false);
  const [phoneNumber, setPhoneNumber] = useState("");
  const [country, setCountry] = useState("");
  const [organization, setOrganization] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [canResend, setCanResend] = useState(false);
  const [verificationUrl, setVerificationUrl] = useState("");
  const [isVerified, setIsVerified] = useState(false);

  useEffect(() => {
    let mounted = true;
    void getAuthCapabilities()
      .then((capabilities) => {
        if (mounted) {
          setEnabled(capabilities.signup_enabled);
        }
      })
      .catch(() => {
        if (mounted) {
          setEnabled(false);
          const message = "Registration configuration could not be loaded. Try again later.";
          setError(message);
          toast({ tone: "error", title: "Registration unavailable", message });
        }
      });
    return () => {
      mounted = false;
    };
  }, [toast]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setMessage("");
    const showValidationError = (message: string) => {
      setError(message);
      toast({ tone: "error", title: "Check your details", message });
    };
    if (firstName.trim().length < 2 || firstName.trim().length > 50) {
      showValidationError("First name must be 2 to 50 characters.");
      return;
    }
    if (lastName.trim().length < 2 || lastName.trim().length > 50) {
      showValidationError("Last name must be 2 to 50 characters.");
      return;
    }
    if (passwordStrength < 5) {
      showValidationError(
        "Password must include uppercase, lowercase, number, and special character.",
      );
      return;
    }
    if (password !== confirmPassword) {
      showValidationError("Passwords do not match.");
      return;
    }
    if (!termsAccepted) {
      showValidationError("Accept the terms and conditions to continue.");
      return;
    }
    setBusy(true);
    try {
      const response = await signUp({
        first_name: firstName,
        last_name: lastName,
        email,
        password,
        confirm_password: confirmPassword,
        terms_accepted: termsAccepted,
        phone_number: phoneNumber || undefined,
        country: country || undefined,
        organization_name: organization || undefined,
      });
      setMessage(response.message);
      toast({
        tone: "success",
        title: response.status === "verified" ? "Account ready" : "Account created",
        message: response.message,
      });
      if (response.verification_url) {
        setVerificationUrl(response.verification_url);
      }

      if (response.status === "verified") {
        setIsVerified(true);
      }
      setCanResend(true);
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : "Registration failed.";
      setError(message);
      toast({
        tone: "error",
        title:
          reason instanceof ApiError && reason.category === "validation"
            ? "Check your details"
            : "Registration failed",
        message,
      });
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
      const message = "If verification is still needed, a new link has been sent.";
      setMessage(message);
      toast({ tone: "success", title: "Verification email sent", message });
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : "The request failed.";
      setError(message);
      toast({ tone: "error", title: "Verification email failed", message });
    } finally {
      setBusy(false);
    }

  }

  const passwordChecks = [
    password.length >= 8,
    /[A-Z]/.test(password),
    /[a-z]/.test(password),
    /\d/.test(password),
    /[^A-Za-z0-9]/.test(password),
  ];
  const passwordStrength = passwordChecks.filter(Boolean).length;

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
          {isVerified ? (
            <Link
              className="inline-flex min-h-11 w-full items-center justify-center rounded-lg bg-emerald-700 px-4 text-sm font-semibold text-white hover:bg-emerald-800"
              href="/login"
            >
              Sign in now &rarr;
            </Link>
          ) : verificationUrl ? (
            <div className="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-xs text-emerald-900">
              <span className="font-semibold">Development mode:</span>{" "}
              <a href={verificationUrl} className="font-bold text-emerald-800 underline">
                Click here to verify email
              </a>
            </div>
          ) : null}
          {!isVerified && canResend && (
            <button
              className="button button-quiet"
              disabled={busy || !email}
              onClick={() => void resend()}
              type="button"
            >
              {busy ? "Sending…" : "Resend verification email"}
            </button>
          )}
          <p className="text-sm text-zinc-600">
            Already verified? <Link className="font-medium text-emerald-800 underline" href="/login">Sign in</Link>
          </p>
        </div>
      ) : (
        <div className="space-y-3">
        <form className="space-y-3" onSubmit={(event) => void submit(event)}>
          <div className="grid gap-3 sm:grid-cols-2">
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            First name
            <input autoComplete="given-name" className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm" maxLength={50} minLength={2} onChange={(event) => setFirstName(event.target.value)} required value={firstName} />
          </label>
          <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Last name
            <input autoComplete="family-name" className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm" maxLength={50} minLength={2} onChange={(event) => setLastName(event.target.value)} required value={lastName} />
          </label>
          </div>
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
              value={organization}
            />
          </label>
            <div className="grid gap-3 sm:grid-cols-2">
            <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
              Phone number <span className="font-normal text-zinc-500">(optional)</span>
              <input autoComplete="tel" className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm" onChange={(event) => setPhoneNumber(event.target.value)} value={phoneNumber} />
            </label>
            <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
              Country <span className="font-normal text-zinc-500">(optional)</span>
              <input autoComplete="country-name" className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm" onChange={(event) => setCountry(event.target.value)} value={country} />
            </label>
            </div>
            <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
            Password
            <input
              autoComplete="new-password"
              className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm"
              minLength={8}
              onChange={(event) => setPassword(event.target.value)}
              required
              type="password"
              value={password}
            />
              <span className="block text-xs font-normal text-zinc-500">
                Strength: {["", "weak", "weak", "fair", "good", "strong"][passwordStrength] || "weak"}.
                {" "}Use 8+ characters with uppercase, lowercase, number, and symbol.
              </span>
            </label>
            <label className="block space-y-1.5 text-sm font-medium text-zinc-700">
              Confirm password
              <input autoComplete="new-password" className="h-10 w-full rounded-lg border border-zinc-300 px-3 text-sm" onChange={(event) => setConfirmPassword(event.target.value)} required type="password" value={confirmPassword} />
            </label>
            <label className="flex items-start gap-2 text-sm text-zinc-700">
              <input checked={termsAccepted} className="mt-1" onChange={(event) => setTermsAccepted(event.target.checked)} required type="checkbox" />
              <span>I agree to the SMTP Platform terms and conditions.</span>
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
