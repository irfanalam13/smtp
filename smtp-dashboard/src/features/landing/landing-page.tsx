"use client";

import Link from "next/link";
import { useState } from "react";
import {
  CheckCircleIcon,
  CheckIcon,
  CopyIcon,
  ExternalLinkIcon,
  GlobeIcon,
  HomeIcon,
  KeyIcon,
  MenuIcon,
  ServerIcon,
  ShieldIcon,
  UserIcon,
  UsersIcon,
  XIcon,
} from "@/components/ui/icons";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

type CodeTab = "python" | "curl" | "node";

const codeSnippets: Record<
  CodeTab,
  { label: string; code: string; language: string }
> = {
  python: {
    label: "Python (SMTP + STARTTLS)",
    language: "python",
    code: `import smtplib
from email.message import EmailMessage

msg = EmailMessage()
msg.set_content("Hello from the high-throughput SMTP Platform!")
msg["Subject"] = "Welcome to SMTP Platform"
msg["From"] = "system@verified-domain.com"
msg["To"] = "recipient@example.com"

# Connect to the high-performance Rust ingress listener
with smtplib.SMTP("127.0.0.1", 2525) as server:
    server.ehlo()
    server.starttls()
    server.ehlo()
    server.login("tenant-username", "secure-smtp-password")
    server.send_message(msg)
    print("Email securely queued in durable PostgreSQL queue.")`,
  },
  curl: {
    label: "cURL (REST API Gateway)",
    language: "bash",
    code: `curl -X POST "http://127.0.0.1:8080/v1/submissions" \\
  -H "Authorization: Bearer stp_live_secret_key" \\
  -H "Content-Type: application/json" \\
  -d '{
    "sender": "alerts@verified-domain.com",
    "recipients": ["user@example.com"],
    "subject": "Critical Cluster Update",
    "body_text": "Your infrastructure has been upgraded successfully.",
    "idempotency_key": "submission-8f92-acb4"
  }'`,
  },
  node: {
    label: "Node.js (Nodemailer)",
    language: "javascript",
    code: `import nodemailer from "nodemailer";

const transporter = nodemailer.createTransport({
  host: "127.0.0.1",
  port: 2525,
  secure: false, // upgrades via STARTTLS
  auth: {
    user: "tenant-username",
    pass: "secure-smtp-password",
  },
});

const info = await transporter.sendMail({
  from: '"Operations Team" <ops@verified-domain.com>',
  to: "developer@example.com",
  subject: "Transactional Event",
  text: "ACID delivery confirmed with lease-based retries.",
});

console.log("Accepted with Message ID:", info.messageId);`,
  },
};

export function LandingPage() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<CodeTab>("python");
  const [copied, setCopied] = useState(false);

  async function copyCurrentCode() {
    try {
      await navigator.clipboard.writeText(codeSnippets[activeTab].code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Ignore clipboard fallback
    }
  }

  return (
    <div className="min-h-screen bg-zinc-50/60 text-zinc-900 selection:bg-emerald-500 selection:text-white">
      {/* Top Banner */}
      <div className="bg-emerald-950 px-4 py-2 text-center text-xs font-medium text-emerald-200">
        <span>
          🚀 SMTP Platform Foundation &mdash; Rust Protocol Engines, Go Control
          Plane & PostgreSQL Queue
        </span>
        <Link
          href="/home"
          className="ml-2 font-semibold text-white underline underline-offset-2 hover:text-emerald-100"
        >
          Open Console &rarr;
        </Link>
      </div>

      {/* Main Header / Navigation */}
      <header className="sticky top-0 z-40 border-b border-zinc-200/80 bg-white/90 backdrop-blur-md">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3 sm:px-6 lg:px-8">
          <div className="flex items-center gap-6">
            <Link href="/" className="flex items-center gap-2.5 group">
              <span className="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-700 text-base font-bold text-white shadow-sm transition-transform group-hover:scale-105">
                S
              </span>
              <div>
                <span className="font-bold tracking-tight text-zinc-900">
                  SMTP Platform
                </span>
                <span className="ml-1.5 hidden rounded-md bg-emerald-50 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-800 border border-emerald-200/70 sm:inline-block">
                  Cloud Core
                </span>
              </div>
            </Link>

            <nav className="hidden items-center gap-5 text-sm font-medium text-zinc-600 md:flex">
              <a
                href="#overview"
                className="transition-colors hover:text-emerald-700"
              >
                Project Brief
              </a>
              <a
                href="#architecture"
                className="transition-colors hover:text-emerald-700"
              >
                Architecture
              </a>
              <a
                href="#auth-portal"
                className="transition-colors hover:text-emerald-700"
              >
                Auth System
              </a>
              <a
                href="#services"
                className="transition-colors hover:text-emerald-700"
              >
                Services
              </a>
              <a
                href="#quickstart"
                className="transition-colors hover:text-emerald-700"
              >
                Quick Start
              </a>
              <a
                href="#deliverability"
                className="transition-colors hover:text-emerald-700"
              >
                Deliverability
              </a>
            </nav>
          </div>

          {/* Desktop Auth & Action Buttons */}
          <div className="hidden items-center gap-2.5 sm:flex">
            <Link
              href="/login"
              className="inline-flex h-9 items-center justify-center rounded-lg border border-zinc-200 bg-white px-3.5 text-xs font-semibold text-zinc-700 shadow-xs transition hover:bg-zinc-50 hover:text-zinc-900 focus-visible:outline-2 focus-visible:outline-emerald-600"
            >
              Sign In
            </Link>
            <Link
              href="/signup"
              className="inline-flex h-9 items-center justify-center rounded-lg bg-emerald-700 px-3.5 text-xs font-semibold text-white shadow-xs transition hover:bg-emerald-800 focus-visible:outline-2 focus-visible:outline-emerald-600"
            >
              Create Account
            </Link>
            <Link
              href="/home"
              className="inline-flex h-9 items-center justify-center gap-1.5 rounded-lg border border-emerald-700/20 bg-emerald-50/70 px-3.5 text-xs font-semibold text-emerald-900 transition hover:bg-emerald-100/70 focus-visible:outline-2 focus-visible:outline-emerald-600"
            >
              <HomeIcon className="h-3.5 w-3.5 text-emerald-700" />
              Console
            </Link>
          </div>

          {/* Mobile Menu Button */}
          <button
            type="button"
            className="flex h-9 w-9 items-center justify-center rounded-lg border border-zinc-200 text-zinc-700 md:hidden"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            aria-label="Toggle navigation"
          >
            {mobileMenuOpen ? (
              <XIcon className="h-5 w-5" />
            ) : (
              <MenuIcon className="h-5 w-5" />
            )}
          </button>
        </div>

        {/* Mobile Navigation Drawer */}
        {mobileMenuOpen && (
          <div className="border-b border-zinc-200 bg-white px-4 py-4 md:hidden">
            <nav className="flex flex-col space-y-3 text-sm font-medium text-zinc-700">
              <a
                href="#overview"
                onClick={() => setMobileMenuOpen(false)}
                className="py-1 hover:text-emerald-700"
              >
                Project Brief
              </a>
              <a
                href="#architecture"
                onClick={() => setMobileMenuOpen(false)}
                className="py-1 hover:text-emerald-700"
              >
                Architecture
              </a>
              <a
                href="#auth-portal"
                onClick={() => setMobileMenuOpen(false)}
                className="py-1 hover:text-emerald-700"
              >
                Auth System
              </a>
              <a
                href="#services"
                onClick={() => setMobileMenuOpen(false)}
                className="py-1 hover:text-emerald-700"
              >
                Services
              </a>
              <a
                href="#quickstart"
                onClick={() => setMobileMenuOpen(false)}
                className="py-1 hover:text-emerald-700"
              >
                Quick Start
              </a>
            </nav>
            <div className="mt-4 flex flex-col gap-2 pt-4 border-t border-zinc-100">
              <Link
                href="/login"
                className="flex h-10 items-center justify-center rounded-lg border border-zinc-300 text-sm font-semibold text-zinc-800"
              >
                Sign In
              </Link>
              <Link
                href="/signup"
                className="flex h-10 items-center justify-center rounded-lg bg-emerald-700 text-sm font-semibold text-white"
              >
                Create Account
              </Link>
              <Link
                href="/home"
                className="flex h-10 items-center justify-center rounded-lg bg-emerald-50 text-sm font-semibold text-emerald-900 border border-emerald-200"
              >
                Launch Console
              </Link>
            </div>
          </div>
        )}
      </header>

      {/* Hero Section */}
      <section className="relative overflow-hidden border-b border-zinc-200/80 bg-gradient-to-b from-white via-zinc-50/50 to-zinc-100/40 py-16 sm:py-24">
        {/* Subtle background decoration */}
        <div
          aria-hidden="true"
          className="pointer-events-none absolute -top-40 left-1/2 -z-10 h-[550px] w-[900px] -translate-x-1/2 rounded-full bg-radial from-emerald-100/60 to-transparent blur-3xl opacity-70"
        />

        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="text-center">
            {/* Pill Badge */}
            <div className="inline-flex items-center gap-2 rounded-full border border-emerald-200/80 bg-emerald-50/80 px-3.5 py-1 text-xs font-semibold text-emerald-800 shadow-xs mb-6">
              <span className="flex h-2 w-2 rounded-full bg-emerald-600 animate-pulse" />
              <span>
                Cloud-Native Rust Protocol Workspaces &bull; Go Control Plane
                &bull; PostgreSQL 17
              </span>
            </div>

            {/* Main Headline */}
            <h1 className="mx-auto max-w-4xl text-3xl font-extrabold tracking-tight text-zinc-950 sm:text-5xl sm:leading-[1.15]">
              High-Assurance SMTP Cloud Infrastructure with{" "}
              <span className="text-emerald-700 bg-gradient-to-r from-emerald-700 to-teal-800 bg-clip-text text-transparent">
                Guaranteed Isolation & Durable Delivery
              </span>
            </h1>

            {/* Subtitle */}
            <p className="mx-auto mt-6 max-w-2xl text-base text-zinc-600 sm:text-lg sm:leading-relaxed">
              Decoupling SMTP submission, durable ACID queuing, outbound
              transport, and mailbox management into memory-safe Rust protocol
              engines and an audited Go control plane.
            </p>

            {/* Primary Action Buttons */}
            <div className="mt-8 flex flex-wrap items-center justify-center gap-3 sm:gap-4">
              <Link
                href="/login"
                className="inline-flex h-12 items-center justify-center gap-2 rounded-xl bg-emerald-700 px-6 text-sm font-semibold text-white shadow-sm transition hover:bg-emerald-800 hover:shadow active:scale-[0.99] focus-visible:outline-2 focus-visible:outline-emerald-600"
              >
                <UserIcon className="h-4 w-4" />
                Sign In to Platform
              </Link>

              <Link
                href="/signup"
                className="inline-flex h-12 items-center justify-center gap-2 rounded-xl border border-zinc-300 bg-white px-6 text-sm font-semibold text-zinc-800 shadow-xs transition hover:bg-zinc-50 hover:text-zinc-950 active:scale-[0.99] focus-visible:outline-2 focus-visible:outline-zinc-400"
              >
                <ShieldIcon className="h-4 w-4 text-emerald-700" />
                Register Organization
              </Link>

              <Link
                href="/home"
                className="inline-flex h-12 items-center justify-center gap-2 rounded-xl border border-emerald-700/20 bg-emerald-50 px-6 text-sm font-semibold text-emerald-900 transition hover:bg-emerald-100 active:scale-[0.99] focus-visible:outline-2 focus-visible:outline-emerald-600"
              >
                <HomeIcon className="h-4 w-4 text-emerald-700" />
                Open Control Plane
              </Link>
            </div>

            {/* Quick Auth Redirection Jump Bar */}
            <div className="mt-8 inline-flex flex-wrap items-center justify-center gap-2 rounded-2xl border border-zinc-200/90 bg-white/80 p-2 shadow-xs backdrop-blur-xs text-xs text-zinc-600 sm:gap-4 sm:px-4">
              <span className="font-semibold text-zinc-500 uppercase tracking-wider text-[11px]">
                Quick Auth Links:
              </span>
              <Link
                href="/login"
                className="font-medium text-emerald-800 hover:text-emerald-950 hover:underline"
              >
                Email/Password Sign-in
              </Link>
              <span className="text-zinc-300">&bull;</span>
              <Link
                href="/signup"
                className="font-medium text-emerald-800 hover:text-emerald-950 hover:underline"
              >
                Tenant Signup
              </Link>
              <span className="text-zinc-300">&bull;</span>
              <Link
                href="/verify-email"
                className="font-medium text-emerald-800 hover:text-emerald-950 hover:underline"
              >
                Email Verification
              </Link>
              <span className="text-zinc-300">&bull;</span>
              <Link
                href="/forgot-password"
                className="font-medium text-emerald-800 hover:text-emerald-950 hover:underline"
              >
                Forgot Password
              </Link>
            </div>
          </div>

          {/* Key Value Highlights Grid */}
          <div className="mt-14 grid grid-cols-2 gap-4 sm:grid-cols-4 lg:gap-6">
            <div className="rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xs">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-700 border border-emerald-200/60">
                  <ServerIcon className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-bold text-zinc-900">Rust Core</h4>
                  <p className="text-xs text-zinc-500">
                    Zero-copy parsing & async I/O
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xs">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-700 border border-emerald-200/60">
                  <ShieldIcon className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-bold text-zinc-900">
                    ACID Queue
                  </h4>
                  <p className="text-xs text-zinc-500">
                    Lease claims & bounded retry
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xs">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-700 border border-emerald-200/60">
                  <GlobeIcon className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-bold text-zinc-900">
                    DNS Diagnostics
                  </h4>
                  <p className="text-xs text-zinc-500">
                    SPF, DKIM, DMARC, MX checks
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-2xl border border-zinc-200 bg-white p-5 shadow-2xs">
              <div className="flex items-center gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-700 border border-emerald-200/60">
                  <KeyIcon className="h-5 w-5" />
                </div>
                <div>
                  <h4 className="text-lg font-bold text-zinc-900">
                    Zero Trust
                  </h4>
                  <p className="text-xs text-zinc-500">Argon2id & OIDC SSO</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Project Brief Section */}
      <section
        id="overview"
        className="border-b border-zinc-200/80 bg-white py-16 sm:py-20"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-3xl text-center">
            <Badge variant="info">Project Brief & Problem Space</Badge>
            <h2 className="mt-3 text-2xl font-bold tracking-tight text-zinc-900 sm:text-3xl">
              Why We Engineered the SMTP Platform
            </h2>
            <p className="mt-4 text-sm leading-relaxed text-zinc-600 sm:text-base">
              Traditional mail servers tightly couple network reception, local
              delivery, disk spooling, and administrative configurations. In a
              multi-tenant cloud environment, a single blocked delivery queue or
              compromised credential can cause cross-tenant latency, security
              violations, and reputation degradation.
            </p>
          </div>

          <div className="mt-12 grid gap-8 md:grid-cols-3">
            <div className="relative rounded-2xl border border-zinc-200 bg-zinc-50/70 p-6 sm:p-8">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-100 text-emerald-800 font-bold mb-4">
                1
              </div>
              <h3 className="text-lg font-bold text-zinc-900">
                Decoupled Service Boundaries
              </h3>
              <p className="mt-2 text-sm leading-6 text-zinc-600">
                Network submission (
                <code className="rounded bg-zinc-200 px-1 py-0.5 text-xs">
                  smtp-session
                </code>
                ), queue persistence (
                <code className="rounded bg-zinc-200 px-1 py-0.5 text-xs">
                  smtp-queue
                </code>
                ), and outbound transport (
                <code className="rounded bg-zinc-200 px-1 py-0.5 text-xs">
                  smtp-delivery
                </code>
                ) are discrete binaries. A surge in incoming submissions never
                starves outbound delivery workers.
              </p>
            </div>

            <div className="relative rounded-2xl border border-zinc-200 bg-zinc-50/70 p-6 sm:p-8">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-100 text-emerald-800 font-bold mb-4">
                2
              </div>
              <h3 className="text-lg font-bold text-zinc-900">
                Strict Tenant Isolation
              </h3>
              <p className="mt-2 text-sm leading-6 text-zinc-600">
                Every API key, SMTP client password, domain check, and mailbox
                is strictly partitioned by tenant ID in PostgreSQL. Ingress
                rejects submissions claiming unverified sender domains,
                preventing tenant spoofing by design.
              </p>
            </div>

            <div className="relative rounded-2xl border border-zinc-200 bg-zinc-50/70 p-6 sm:p-8">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-100 text-emerald-800 font-bold mb-4">
                3
              </div>
              <h3 className="text-lg font-bold text-zinc-900">
                Durable Leased Queueing
              </h3>
              <p className="mt-2 text-sm leading-6 text-zinc-600">
                Rather than relying on volatile spool folders, messages enter an
                ACID PostgreSQL queue with unique idempotency keys. Delivery
                workers claim messages with time-bounded leases, guaranteeing
                at-least-once delivery without duplicate deliveries during
                crashes.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Authentication System Portal Section (Core Requirement) */}
      <section
        id="auth-portal"
        className="border-b border-zinc-200/80 bg-zinc-100/50 py-16 sm:py-20"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-3xl text-center">
            <Badge variant="success">Authentication Gateway</Badge>
            <h2 className="mt-3 text-2xl font-bold tracking-tight text-zinc-900 sm:text-4xl">
              Enterprise Identity & Access System
            </h2>
            <p className="mt-3 text-sm text-zinc-600 sm:text-base">
              Choose your authentication pathway. The platform supports
              password-based tenant logins with Argon2id, federated OIDC single
              sign-on, and scoped developer credentials.
            </p>
          </div>

          {/* Auth System Cards Grid */}
          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {/* Card 1: Password Login */}
            <Card className="flex flex-col justify-between border-emerald-200/80 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-50 text-emerald-800 border border-emerald-200">
                    <UserIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="success">Primary Auth</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">
                  Tenant Account Sign-In
                </CardTitle>
                <CardDescription>
                  Authenticate into your workspace using your verified email
                  address and organization credentials.
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                Protected by Argon2id hashing, rate limiting, and sliding
                session token cookies.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/login"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg bg-emerald-700 py-2.5 text-xs font-semibold text-white transition hover:bg-emerald-800"
                >
                  Go to Sign In &rarr;
                </Link>
              </CardFooter>
            </Card>

            {/* Card 2: Tenant Signup */}
            <Card className="flex flex-col justify-between border-zinc-200 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-blue-50 text-blue-800 border border-blue-200">
                    <UsersIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="info">Registration</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">
                  Create Organization
                </CardTitle>
                <CardDescription>
                  Provision a new isolated tenant workspace, setup your
                  administrative user, and receive an activation token.
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                Initializes multi-tenant database constraints and scoped
                permission envelopes.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/signup"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-300 bg-white py-2.5 text-xs font-semibold text-zinc-800 transition hover:bg-zinc-50 hover:text-zinc-950"
                >
                  Register Account &rarr;
                </Link>
              </CardFooter>
            </Card>

            {/* Card 3: OIDC Single Sign-On */}
            <Card className="flex flex-col justify-between border-zinc-200 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-purple-50 text-purple-800 border border-purple-200">
                    <ShieldIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="neutral">Enterprise SSO</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">
                  OIDC Identity Provider
                </CardTitle>
                <CardDescription>
                  Federated sign-in via OpenID Connect (Keycloak, Okta, Google
                  Workspace, Azure AD).
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                Cryptographic JWKS validation with automated subject and tenant
                claim binding.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/login"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-purple-200 bg-purple-50/60 py-2.5 text-xs font-semibold text-purple-900 transition hover:bg-purple-100/80"
                >
                  SSO Sign In &rarr;
                </Link>
              </CardFooter>
            </Card>

            {/* Card 4: Email Verification */}
            <Card className="flex flex-col justify-between border-zinc-200 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-amber-50 text-amber-800 border border-amber-200">
                    <CheckCircleIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="warning">Activation</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">
                  Email Verification
                </CardTitle>
                <CardDescription>
                  Confirm account ownership using the cryptographic verification
                  token sent upon registration.
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                Activation state is stored atomically in PostgreSQL before
                credentials are fully unlocked.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/verify-email"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-300 bg-white py-2.5 text-xs font-semibold text-zinc-800 transition hover:bg-zinc-50"
                >
                  Verify Email Address &rarr;
                </Link>
              </CardFooter>
            </Card>

            {/* Card 5: Password Recovery */}
            <Card className="flex flex-col justify-between border-zinc-200 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-rose-50 text-rose-800 border border-rose-200">
                    <KeyIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="error">Recovery</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">Password Reset</CardTitle>
                <CardDescription>
                  Request a time-bounded, single-use password reset link to
                  securely restore access to your account.
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                Invalidates existing active sessions upon password replacement.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/forgot-password"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-zinc-300 bg-white py-2.5 text-xs font-semibold text-zinc-800 transition hover:bg-zinc-50"
                >
                  Reset Password &rarr;
                </Link>
              </CardFooter>
            </Card>

            {/* Card 6: Developer Credentials */}
            <Card className="flex flex-col justify-between border-zinc-200 bg-white shadow-xs transition hover:shadow-md">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-teal-50 text-teal-800 border border-teal-200">
                    <ServerIcon className="h-5 w-5" />
                  </div>
                  <Badge variant="neutral">API & SMTP</Badge>
                </div>
                <CardTitle className="mt-4 text-lg">
                  Machine Credentials
                </CardTitle>
                <CardDescription>
                  Generate TLS SMTP AUTH credentials and scoped API keys for
                  automated backend pipelines.
                </CardDescription>
              </CardHeader>
              <CardContent className="text-xs text-zinc-500">
                API keys are hashed with SHA-256 and SMTP passwords hashed with
                Argon2id.
              </CardContent>
              <CardFooter className="pt-4">
                <Link
                  href="/account/smtp-credentials"
                  className="inline-flex w-full items-center justify-center gap-2 rounded-lg border border-teal-200 bg-teal-50/70 py-2.5 text-xs font-semibold text-teal-900 transition hover:bg-teal-100"
                >
                  Manage SMTP Credentials &rarr;
                </Link>
              </CardFooter>
            </Card>
          </div>
        </div>
      </section>

      {/* Architecture Deep Dive Section */}
      <section
        id="architecture"
        className="border-b border-zinc-200/80 bg-white py-16 sm:py-20"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-3xl text-center">
            <Badge variant="info">Monorepo Topology</Badge>
            <h2 className="mt-3 text-2xl font-bold tracking-tight text-zinc-900 sm:text-3xl">
              System Architecture & Component Blueprint
            </h2>
            <p className="mt-4 text-sm text-zinc-600 sm:text-base">
              The platform brings together Rust performance, Go concurrency, and
              PostgreSQL transactional durability.
            </p>
          </div>

          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {/* Component: smtp-session */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  smtp-session
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Rust 2024
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Ingress SMTP Listener
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                Implements STARTTLS, implicit TLS, TLS-only AUTH PLAIN, rate
                limits, verified-domain sender check, and inbound message
                fanning across tenant mailboxes.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Port 2525 / 587
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  MIME Parsing
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Tokio Async
                </span>
              </div>
            </div>

            {/* Component: smtp-queue */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  smtp-queue
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Rust 2024
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Durable ACID Queue
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                PostgreSQL-backed queue with idempotent enqueue, lease-based
                worker claims, exponential retry with jitter, and full attempt
                history logs.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Idempotent Keys
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  SQLx / Postgres
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Lease Heartbeat
                </span>
              </div>
            </div>

            {/* Component: smtp-delivery */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  smtp-delivery
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Rust 2024
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Outbound Transport
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                Asynchronous outbound dispatcher with MX record resolution,
                destination safety IP filtering (blocking bogon/private
                subnets), and STARTTLS negotiation.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  MX Resolver
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  TLS 1.3 / Rustls
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Safety Guard
                </span>
              </div>
            </div>

            {/* Component: mailbox-service & smtp-core */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  mailbox-service
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Rust 2024
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Mailbox Core Engine
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                Transactional quota allocation, folders (INBOX, Sent, Trash,
                Drafts, Quarantine), labels, MIME body extraction, and full-text
                search indexing.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Axum REST
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Quota Tracking
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Search Index
                </span>
              </div>
            </div>

            {/* Component: smtp-api */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  smtp-api
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Go 1.26
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Control Plane REST API
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                The management backbone conforming to OpenAPI 3.1. Handles
                tenant provisioning, domain DNS checks, submission ingest, API
                keys, and audit logging.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  OpenAPI 3.1
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  DNS Diagnostics
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Prometheus
                </span>
              </div>
            </div>

            {/* Component: smtp-auth */}
            <div className="rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xs">
              <div className="flex items-center justify-between">
                <span className="font-mono text-xs font-bold text-emerald-800 bg-emerald-50 px-2 py-1 rounded border border-emerald-200">
                  smtp-auth
                </span>
                <span className="text-xs font-semibold text-zinc-400">
                  Go 1.26
                </span>
              </div>
              <h4 className="mt-4 text-base font-bold text-zinc-900">
                Auth & Cryptography
              </h4>
              <p className="mt-2 text-xs leading-5 text-zinc-600">
                Argon2id password hashing bounded by concurrency semaphores,
                constant-time verification, OIDC JWT validation, and
                cryptographically secure session storage.
              </p>
              <div className="mt-4 flex flex-wrap gap-1.5">
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Argon2id
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  OIDC / JWKS
                </span>
                <span className="rounded bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-700">
                  Constant-Time
                </span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Code Examples & Quick Start Section */}
      <section
        id="quickstart"
        className="border-b border-zinc-200/80 bg-zinc-900 py-16 text-zinc-100 sm:py-20"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="grid gap-12 lg:grid-cols-12 lg:items-center">
            <div className="lg:col-span-5">
              <Badge variant="neutral">Developer Quick Start</Badge>
              <h2 className="mt-3 text-2xl font-bold tracking-tight text-white sm:text-3xl">
                Submit Messages in Minutes
              </h2>
              <p className="mt-4 text-sm leading-relaxed text-zinc-400">
                Integrate using standard SMTP over TLS (compatible with any
                library or framework) or leverage our high-throughput REST API
                for cloud-native transactional applications.
              </p>

              <div className="mt-6 space-y-3">
                <div className="flex items-start gap-3">
                  <div className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                    <CheckIcon className="h-3.5 w-3.5" />
                  </div>
                  <span className="text-xs text-zinc-300">
                    Zero client modifications: Standard SMTP port 587/2525 with
                    STARTTLS and AUTH PLAIN
                  </span>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                    <CheckIcon className="h-3.5 w-3.5" />
                  </div>
                  <span className="text-xs text-zinc-300">
                    REST API endpoint with automatic deduplication through
                    idempotency keys
                  </span>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-emerald-500/20 text-emerald-400">
                    <CheckIcon className="h-3.5 w-3.5" />
                  </div>
                  <span className="text-xs text-zinc-300">
                    Automated sender domain authorization and real-time delivery
                    attempt logging
                  </span>
                </div>
              </div>

              <div className="mt-8 flex gap-3">
                <Link
                  href="/home"
                  className="inline-flex items-center justify-center rounded-lg bg-emerald-600 px-4 py-2.5 text-xs font-semibold text-white hover:bg-emerald-500"
                >
                  Explore Dashboard Console
                </Link>
                <Link
                  href="/account/smtp-credentials"
                  className="inline-flex items-center justify-center rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2.5 text-xs font-semibold text-zinc-300 hover:bg-zinc-700"
                >
                  Get SMTP Passwords
                </Link>
              </div>
            </div>

            <div className="lg:col-span-7">
              <div className="rounded-2xl border border-zinc-800 bg-zinc-950 shadow-2xl overflow-hidden">
                {/* Code Window Header */}
                <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-2.5 bg-zinc-950/80">
                  <div className="flex items-center gap-2">
                    <span className="h-3 w-3 rounded-full bg-rose-500/80" />
                    <span className="h-3 w-3 rounded-full bg-amber-500/80" />
                    <span className="h-3 w-3 rounded-full bg-emerald-500/80" />
                    <span className="ml-2 font-mono text-xs text-zinc-400">
                      submission-client.
                      {activeTab === "python"
                        ? "py"
                        : activeTab === "curl"
                          ? "sh"
                          : "ts"}
                    </span>
                  </div>

                  <div className="flex items-center gap-1">
                    {(["python", "curl", "node"] as CodeTab[]).map((tab) => (
                      <button
                        key={tab}
                        type="button"
                        onClick={() => setActiveTab(tab)}
                        className={`rounded-md px-2.5 py-1 text-xs font-medium transition ${
                          activeTab === tab
                            ? "bg-zinc-800 text-emerald-400 font-semibold"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {tab.toUpperCase()}
                      </button>
                    ))}
                    <button
                      type="button"
                      onClick={copyCurrentCode}
                      className="ml-2 flex h-7 items-center gap-1 rounded-md border border-zinc-700 bg-zinc-800/80 px-2 text-xs text-zinc-300 hover:bg-zinc-700"
                      title="Copy code"
                    >
                      {copied ? (
                        <>
                          <CheckIcon className="h-3.5 w-3.5 text-emerald-400" />
                          <span className="text-emerald-400 text-[11px]">
                            Copied
                          </span>
                        </>
                      ) : (
                        <>
                          <CopyIcon className="h-3.5 w-3.5" />
                          <span className="text-[11px]">Copy</span>
                        </>
                      )}
                    </button>
                  </div>
                </div>

                {/* Code Window Body */}
                <pre className="overflow-x-auto p-5 font-mono text-xs leading-6 text-zinc-300">
                  <code>{codeSnippets[activeTab].code}</code>
                </pre>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Deliverability & Compliance Section */}
      <section
        id="deliverability"
        className="border-b border-zinc-200/80 bg-white py-16 sm:py-20"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="mx-auto max-w-3xl text-center">
            <Badge variant="warning">Deliverability Suite</Badge>
            <h2 className="mt-3 text-2xl font-bold tracking-tight text-zinc-900 sm:text-3xl">
              Automated DNS Diagnostics & Domain Verification
            </h2>
            <p className="mt-4 text-sm text-zinc-600 sm:text-base">
              Protect your sender reputation before transmitting a single
              message. The control plane checks DNS records continuously to
              maintain high deliverability standards.
            </p>
          </div>

          <div className="mt-12 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
            <div className="rounded-xl border border-zinc-200 p-5">
              <div className="flex items-center gap-2 font-bold text-zinc-900 text-sm">
                <CheckCircleIcon className="h-4 w-4 text-emerald-600" />
                <span>Domain Ownership</span>
              </div>
              <p className="mt-2 text-xs text-zinc-600 leading-relaxed">
                Unique cryptographic TXT record challenges prevent unauthorized
                tenants from registering or sending under your domain.
              </p>
            </div>

            <div className="rounded-xl border border-zinc-200 p-5">
              <div className="flex items-center gap-2 font-bold text-zinc-900 text-sm">
                <CheckCircleIcon className="h-4 w-4 text-emerald-600" />
                <span>SPF Validation</span>
              </div>
              <p className="mt-2 text-xs text-zinc-600 leading-relaxed">
                Parses SPF TXT records for valid mechanism syntax, lookup
                limits, and required relay authorization tags.
              </p>
            </div>

            <div className="rounded-xl border border-zinc-200 p-5">
              <div className="flex items-center gap-2 font-bold text-zinc-900 text-sm">
                <CheckCircleIcon className="h-4 w-4 text-emerald-600" />
                <span>DMARC Enforcement</span>
              </div>
              <p className="mt-2 text-xs text-zinc-600 leading-relaxed">
                Verifies{" "}
                <code className="bg-zinc-100 px-1 py-0.5 rounded text-[11px]">
                  _dmarc
                </code>{" "}
                records and reports policy alignment (
                <code className="text-zinc-700">p=quarantine</code> or{" "}
                <code className="text-zinc-700">p=reject</code>).
              </p>
            </div>

            <div className="rounded-xl border border-zinc-200 p-5">
              <div className="flex items-center gap-2 font-bold text-zinc-900 text-sm">
                <CheckCircleIcon className="h-4 w-4 text-emerald-600" />
                <span>MX Inbound Routing</span>
              </div>
              <p className="mt-2 text-xs text-zinc-600 leading-relaxed">
                Validates MX records pointing to the ingress cluster for
                seamless mailbox reception and multi-user fan-out.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-zinc-200 bg-zinc-100 py-12 text-zinc-600">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="flex flex-col items-center justify-between gap-6 sm:flex-row">
            <div className="flex items-center gap-2.5">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-700 text-xs font-bold text-white">
                S
              </span>
              <span className="font-bold text-sm text-zinc-900">
                SMTP Platform
              </span>
              <span className="text-xs text-zinc-400">
                &bull; Open Source Foundation
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-6 text-xs font-medium">
              <Link href="/home" className="hover:text-emerald-700">
                Console Home
              </Link>
              <Link href="/login" className="hover:text-emerald-700">
                Sign In
              </Link>
              <Link href="/signup" className="hover:text-emerald-700">
                Create Account
              </Link>
              <Link href="/verify-email" className="hover:text-emerald-700">
                Verify Email
              </Link>
              <Link href="/forgot-password" className="hover:text-emerald-700">
                Reset Password
              </Link>
              <a
                href="https://github.com"
                target="_blank"
                rel="noreferrer"
                className="flex items-center gap-1 hover:text-emerald-700"
              >
                <span>Documentation</span>
                <ExternalLinkIcon className="h-3 w-3" />
              </a>
            </div>
          </div>

          <div className="mt-8 border-t border-zinc-200 pt-6 text-center text-xs text-zinc-500 sm:flex sm:justify-between sm:text-left">
            <p>
              &copy; {new Date().getFullYear()} SMTP Platform. All rights
              reserved.
            </p>
            <p className="mt-2 sm:mt-0">
              Built with Rust, Go, Next.js, and PostgreSQL &bull; Monorepo
              Architecture
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
