"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { RefreshCwIcon } from "@/components/ui/icons";
import { useControlPlaneDashboard } from "../../hooks/use-control-plane-dashboard";
import { useMailboxManagement } from "../../hooks/use-mailbox-management";
import { useSMTPCredentials } from "../../hooks/use-smtp-credentials";
import { SessionManagement } from "@/features/auth/components/session-management";
import { LogoutButton } from "@/features/auth/components/logout-button";
import { ApiKeysView } from "../views/api-keys-view";
import { ActivityView } from "../views/activity-view";
import { DashboardSidebar } from "./dashboard-sidebar";
import { DomainsView } from "../views/domains-view";
import { MailboxesView } from "../views/mailboxes-view";
import { MailboxManagementView } from "../views/mailbox-management-view";
import { OverviewView } from "../views/overview-view";
import { SendEmailView } from "../views/send-email-view";
import { SMTPCredentialsView } from "../views/smtp-credentials-view";
import type { DashboardView } from "../../types/control-plane";

const brandName = process.env.NEXT_PUBLIC_BRAND_NAME?.trim() || "SMTP Platform";
const localDevelopment = process.env.NODE_ENV === "development";

const viewTitles: Record<
  DashboardView,
  { title: string; description: string }
> = {
  home: {
    title: "Home",
    description: "Your email workspace at a glance.",
  },
  send: {
    title: "Send an email",
    description: "Submit a message to the durable queue.",
  },
  activity: {
    title: "Delivery activity",
    description: "Inspect queue processing attempts for recent submissions.",
  },
  domains: {
    title: "Sending domains",
    description: "Register domains and manage DNS verification.",
  },
  mailboxes: {
    title: "Mailboxes",
    description: "Browse mailbox message metadata available to your account.",
  },
  "mailbox-admin": {
    title: "Mailbox administration",
    description: "Manage mailbox lifecycle, access, folders, and messages.",
  },
  "api-keys": {
    title: "API keys",
    description: "Create and revoke scoped credentials.",
  },
  "smtp-credentials": {
    title: "SMTP credentials",
    description: "Manage passwords used by SMTP AUTH clients.",
  },
  credentials: {
    title: "Credentials",
    description: "Connect with an API key or OIDC access token.",
  },
  statistics: {
    title: "Analytics & Statistics",
    description:
      "Inspect deliverability trends, ISP breakdown, and bounce rates.",
  },
  templates: {
    title: "Email Templates",
    description: "Manage and preview dynamic transactional templates.",
  },
  sms: {
    title: "SMS Messaging",
    description: "Send transactional SMS messages and track carrier receipts.",
  },
  suppressions: {
    title: "Suppressions & Bounces",
    description: "Manage automated bounce and complaint suppression lists.",
  },
  "event-webhooks": {
    title: "Event Webhooks",
    description: "Real-time delivery lifecycle webhooks with HMAC signatures.",
  },
  "routing-rules": {
    title: "Inbound Routing Rules",
    description: "Catch-all routing, regex matches, and forwarders.",
  },
  "api-reference": {
    title: "API Reference & SDKs",
    description: "Interactive OpenAPI 3.1 specifications and client libraries.",
  },
  webhooks: {
    title: "Developer Webhooks",
    description: "Developer notification endpoints for account events.",
  },
  team: {
    title: "Team & Access Control",
    description: "Manage members, invites, and role-based permissions.",
  },
  billing: {
    title: "Usage & Billing",
    description: "Monitor daily consumption, storage limits, and plan tiers.",
  },
};

export default function ControlPlaneDashboard() {
  const dashboard = useControlPlaneDashboard();
  const mailboxManagement = useMailboxManagement(
    dashboard.requestContext,
    dashboard.mailboxes,
    dashboard.domains,
    dashboard.view === "mailbox-admin",
    dashboard.load,
  );
  const smtpCredentials = useSMTPCredentials(
    dashboard.requestContext,
    dashboard.view === "smtp-credentials",
  );
  const currentTitle = viewTitles[dashboard.view] ?? {
    title: "Unavailable",
    description:
      "This product area is not available in the current control plane.",
  };
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const [mobileViewport, setMobileViewport] = useState(false);
  const [currentTime, setCurrentTime] = useState<number | null>(null);
  const menuButtonRef = useRef<HTMLButtonElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const wasNavigationOpen = useRef(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(max-width: 800px)");
    const updateViewport = () => {
      setMobileViewport(mediaQuery.matches);
      if (!mediaQuery.matches) {
        setMobileNavigationOpen(false);
      }
    };
    updateViewport();
    mediaQuery.addEventListener("change", updateViewport);
    return () => mediaQuery.removeEventListener("change", updateViewport);
  }, []);

  useEffect(() => {
    const updateTime = () => setCurrentTime(Date.now());
    const initialUpdate = window.setTimeout(updateTime, 0);
    const interval = window.setInterval(updateTime, 60_000);
    return () => {
      window.clearTimeout(initialUpdate);
      window.clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    if (mobileNavigationOpen) {
      const previousOverflow = document.body.style.overflow;
      const handleEscape = (event: KeyboardEvent) => {
        if (event.key === "Escape") {
          event.preventDefault();
          setMobileNavigationOpen(false);
        }
      };
      document.body.style.overflow = "hidden";
      window.addEventListener("keydown", handleEscape);
      wasNavigationOpen.current = true;
      return () => {
        document.body.style.overflow = previousOverflow;
        window.removeEventListener("keydown", handleEscape);
      };
    }
    if (wasNavigationOpen.current && mobileViewport) {
      menuButtonRef.current?.focus();
      wasNavigationOpen.current = false;
    }
  }, [mobileNavigationOpen, mobileViewport]);

  return (
    <div className="app-shell">
      <DashboardSidebar
        key={dashboard.view}
        brandName={brandName}
        view={dashboard.view}
        onViewChange={dashboard.setView}
        mobile={mobileViewport}
        mobileOpen={mobileNavigationOpen}
        onMobileClose={() => setMobileNavigationOpen(false)}
        closeButtonRef={closeButtonRef}
        connected={dashboard.connected}
        loading={dashboard.loading}
        connectionError={dashboard.connectionError}
        principal={dashboard.principal}
        tenant={dashboard.tenant}
      />
      {mobileViewport && mobileNavigationOpen && (
        <button
          aria-label="Close navigation"
          className="sidebar-backdrop"
          onClick={() => setMobileNavigationOpen(false)}
          type="button"
        />
      )}

      <main className="main-panel">
        <header className="topbar">
          <div className="topbar-left">
            <button
              aria-controls="dashboard-sidebar"
              aria-expanded={mobileNavigationOpen}
              aria-label="Open dashboard menu"
              title="Open menu"
              className="mobile-menu-button"
              onClick={() => setMobileNavigationOpen(true)}
              ref={menuButtonRef}
              type="button"
            >
              <span aria-hidden="true" />
              <span aria-hidden="true" />
              <span aria-hidden="true" />
            </button>
            <span className="hidden text-xs font-semibold text-zinc-500 sm:inline">Menu</span>
            <div className="breadcrumb">
              <span>Control plane</span>
              <span className="breadcrumb-separator">/</span>
              <strong>{currentTitle.title}</strong>
            </div>
          </div>
          <div className="topbar-context">
            <span
              className={`topbar-readiness ${dashboard.connected ? "ready" : ""}`}
            >
              <span aria-hidden="true" />
              {dashboard.connected ? "API ready" : "API status unknown"}
            </span>
            <span className="topbar-tenant">
              {dashboard.tenant?.name ??
                dashboard.principal?.tenant?.name ??
                "No workspace"}
            </span>
            <button
              aria-label="Refresh dashboard data"
              className="icon-button"
              disabled={dashboard.loading}
              onClick={() => void dashboard.load()}
              type="button"
            >
              <RefreshCwIcon aria-hidden="true" />
            </button>
            {dashboard.principal?.subject.startsWith("session:") && (
              <LogoutButton />
            )}
          </div>
        </header>

        <div className="content">
          <div className="page-heading">
            <div>
              <p className="eyebrow">SMTP PLATFORM</p>
              <h1>{currentTitle.title}</h1>
              <p>{currentTitle.description}</p>
            </div>
          </div>

          {dashboard.view === "credentials" && (
            <>
            <section className="auth-panel">
              <div className="auth-copy">
                <span className="auth-lock" aria-hidden="true">
                  KEY
                </span>
                <span>
                  <strong>
                    {dashboard.apiToken
                      ? "Credentials active"
                      : localDevelopment
                        ? "Local development authentication"
                        : "Connect your account"}
                  </strong>
                  <small>
                    {dashboard.apiToken
                      ? "The token is held in memory only and cleared when the page reloads."
                      : localDevelopment
                        ? "The dashboard sends a temporary local identity; the API must have CONTROL_PLANE_DEV_MODE=true to accept it. Otherwise, paste a durable user-bound key."
                        : "Paste a user-bound API key or OIDC access token to connect."}
                    {dashboard.principalError || dashboard.connectionError
                      ? ` ${dashboard.principalError || dashboard.connectionError}`
                      : ""}
                  </small>
                </span>
              </div>
              <form className="token-form" onSubmit={dashboard.applyToken}>
                <label className="visually-hidden" htmlFor="api-token">
                  API key or OIDC access token
                </label>
                <input
                  autoComplete="off"
                  id="api-token"
                  onChange={(event) =>
                    dashboard.setTokenInput(event.target.value)
                  }
                  placeholder="Paste API key or access token"
                  type="password"
                  value={dashboard.tokenInput}
                />
                <label className="visually-hidden" htmlFor="tenant-id">
                  Tenant ID for multi-tenant accounts
                </label>
                <input
                  autoComplete="off"
                  className="tenant-input"
                  id="tenant-id"
                  onChange={(event) =>
                    dashboard.setTenantInput(event.target.value)
                  }
                  placeholder="Tenant ID (optional)"
                  value={dashboard.tenantInput}
                />
                <button className="button button-primary" type="submit">
                  Connect
                </button>
                {dashboard.apiToken && (
                  <button
                    className="button button-quiet"
                    onClick={dashboard.clearToken}
                    type="button"
                  >
                    Clear
                  </button>
                )}
              </form>
              <Link
                className="text-xs font-medium text-emerald-800 underline underline-offset-4"
                href="/login"
              >
                Organization sign-in
              </Link>
            </section>
            {dashboard.principal?.subject.startsWith("session:") && (
              <SessionManagement context={dashboard.requestContext} />
            )}
            </>
          )}

          {dashboard.view !== "home" && dashboard.view !== "credentials" && (
            <div className="notice notice-info">
              <span className="notice-mark" aria-hidden="true">
                i
              </span>
              <p>
                <strong>Queue processing is in test mode.</strong> Submissions
                are saved to PostgreSQL, but the current worker simulates
                delivery and does not send email to the internet.
              </p>
            </div>
          )}

          {dashboard.actionError && (
            <div className="notice notice-error" role="alert">
              <span className="notice-mark" aria-hidden="true">
                !
              </span>
              <p>{dashboard.actionError}</p>
              <button
                aria-label="Dismiss error"
                className="dismiss-button"
                onClick={() => dashboard.setActionError("")}
                type="button"
              >
                ×
              </button>
            </div>
          )}

          {dashboard.actionMessage && (
            <div className="notice notice-success" role="status">
              <span className="notice-mark" aria-hidden="true">
                ✓
              </span>
              <p>{dashboard.actionMessage}</p>
              <button
                aria-label="Dismiss message"
                className="dismiss-button"
                onClick={() => dashboard.setActionMessage("")}
                type="button"
              >
                ×
              </button>
            </div>
          )}

          {dashboard.view === "home" && (
            <OverviewView
              domains={dashboard.domains}
              verifiedDomains={dashboard.verifiedDomains}
              keys={dashboard.keys}
              submissions={dashboard.submissions}
              usage={dashboard.usage}
              principal={dashboard.principal}
              tenant={dashboard.tenant}
              readiness={dashboard.readiness}
              currentTime={currentTime}
              loading={dashboard.loading}
              loadErrors={dashboard.loadErrors}
              onRetry={() => void dashboard.load()}
              onCompose={() => dashboard.setView("send")}
              onActivity={() => dashboard.setView("activity")}
              onDomains={() => dashboard.setView("domains")}
              onApiKeys={() => dashboard.setView("api-keys")}
            />
          )}
          {dashboard.view === "send" && (
            <SendEmailView
              sender={dashboard.sender}
              recipients={dashboard.recipients}
              cc={dashboard.cc}
              bcc={dashboard.bcc}
              sendingConfig={dashboard.sendingConfig}
              subject={dashboard.subject}
              body={dashboard.body}
              sending={dashboard.sending}
              loading={dashboard.loading}
              submissions={dashboard.submissions}
              loadErrors={dashboard.loadErrors}
              composeErrors={dashboard.composeErrors}
              onRetry={() => void dashboard.load()}
              onSenderChange={dashboard.setSender}
              onRecipientsChange={dashboard.setRecipients}
              onCcChange={dashboard.setCc}
              onBccChange={dashboard.setBcc}
              onSubjectChange={dashboard.setSubject}
              onBodyChange={dashboard.setBody}
              onSubmit={dashboard.sendMessage}
            />
          )}
          {dashboard.view === "activity" && (
            <ActivityView
              submissions={dashboard.submissions}
              selectedSubmissionId={dashboard.activitySubmissionId}
              attempts={dashboard.submissionAttempts}
              loading={dashboard.loading}
              attemptsLoading={dashboard.activityLoading}
              submissionsError={dashboard.loadErrors.submissions}
              attemptsError={dashboard.activityError}
              onRetrySubmissions={() => void dashboard.load()}
              onRetryAttempts={() => {
                if (dashboard.activitySubmissionId) {
                  void dashboard.loadSubmissionAttempts(
                    dashboard.activitySubmissionId,
                  );
                }
              }}
              onSelectSubmission={(submissionId) =>
                void dashboard.loadSubmissionAttempts(submissionId)
              }
            />
          )}
          {dashboard.view === "domains" && (
            <DomainsView
              domains={dashboard.domains}
              domainInput={dashboard.domainInput}
              domainInputError={dashboard.domainInputError}
              loading={dashboard.loading}
              error={dashboard.loadErrors.domains}
              challenges={dashboard.challenges}
              domainChecks={dashboard.domainChecks}
              domainReadiness={dashboard.domainReadiness}
              domainDetailsLoading={dashboard.domainDetailsLoading}
              domainDetailsErrors={dashboard.domainDetailsErrors}
              busyAction={dashboard.busyAction}
              onDomainInputChange={dashboard.setDomainInput}
              onAddDomain={dashboard.addDomain}
              onCreateChallenge={(domainId) =>
                void dashboard.createChallenge(domainId)
              }
              onRunChecks={(domainId) =>
                void dashboard.runDomainChecks(domainId)
              }
              onConfirmOwnership={(domainId) =>
                void dashboard.confirmOwnership(domainId)
              }
              onDisableDomain={(domain) => void dashboard.disableDomain(domain)}
              onLoadDomainDetails={(domainId) =>
                void dashboard.loadDomainDetails(domainId)
              }
              onRetry={() => void dashboard.load()}
            />
          )}
          {dashboard.view === "mailboxes" && (
            <MailboxesView
              mailboxes={dashboard.mailboxes}
              messages={dashboard.messages}
              selectedMailbox={dashboard.selectedMailbox}
              selectedMessage={dashboard.selectedMessage}
              selectedMessageId={dashboard.selectedMessageId}
              loading={dashboard.loading}
              messagesLoading={dashboard.messagesLoading}
              messageDetailLoading={dashboard.messageDetailLoading}
              error={dashboard.loadErrors.mailboxes}
              messageError={dashboard.messageError}
              onSelectMailbox={(mailboxId) =>
                void dashboard.selectMailbox(mailboxId)
              }
              onSelectMessage={(messageId) =>
                void dashboard.selectMessage(messageId)
              }
              onRetry={() =>
                void dashboard.selectMailbox(dashboard.selectedMailbox)
              }
              onRetryMessage={() =>
                void dashboard.selectMessage(dashboard.selectedMessageId)
              }
            />
          )}
          {dashboard.view === "mailbox-admin" && (
            <MailboxManagementView
              mailboxes={dashboard.mailboxes}
              domains={dashboard.domains}
              management={mailboxManagement}
              onRetry={() => void mailboxManagement.load()}
            />
          )}
          {dashboard.view === "api-keys" && (
            <ApiKeysView
              keys={dashboard.keys}
              keyName={dashboard.keyName}
              keyNameError={dashboard.keyNameError}
              keyScopes={dashboard.keyScopes}
              keyExpiresAt={dashboard.keyExpiresAt}
              keyExpiryPreset={dashboard.keyExpiryPreset}
              oneTimeSecret={dashboard.oneTimeSecret}
              secretWarning={dashboard.secretWarning}
              secretCopied={dashboard.secretCopied}
              loading={dashboard.loading}
              error={dashboard.loadErrors.keys}
              busyAction={dashboard.busyAction}
              currentTime={currentTime}
              onKeyNameChange={dashboard.setKeyName}
              onKeyExpiresAtChange={dashboard.setKeyExpiresAt}
              onKeyExpiryPresetChange={dashboard.setKeyExpiryPreset}
              onToggleScope={dashboard.toggleKeyScope}
              onCreateKey={dashboard.addApiKey}
              onCopySecret={() => void dashboard.copyApiKey()}
              onDismissSecret={dashboard.dismissApiKeySecret}
              onRevokeKey={(keyId) => void dashboard.removeApiKey(keyId)}
              onRetry={() => void dashboard.load()}
            />
          )}
          {dashboard.view === "smtp-credentials" && (
            <SMTPCredentialsView
              credentials={smtpCredentials.credentials}
              loading={smtpCredentials.loading}
              error={smtpCredentials.error}
              username={smtpCredentials.username}
              expiresAt={smtpCredentials.expiresAt}
              busyAction={smtpCredentials.busyAction}
              oneTimeCredential={smtpCredentials.oneTimeCredential}
              secretCopied={smtpCredentials.secretCopied}
              currentTime={currentTime}
              onUsernameChange={smtpCredentials.setUsername}
              onExpiryChange={smtpCredentials.setExpiresAt}
              onCreate={smtpCredentials.create}
              onRotate={(credentialId) =>
                void smtpCredentials.rotate(credentialId)
              }
              onRevoke={(credentialId) =>
                void smtpCredentials.revoke(credentialId)
              }
              onCopy={() => void smtpCredentials.copySecret()}
              onDismiss={smtpCredentials.dismissSecret}
              onRetry={() => void smtpCredentials.reload()}
            />
          )}

          <footer className="page-footer">
            <span>{brandName} Control Plane</span>
            <span>
              {dashboard.principal
                ? `Connected as ${dashboard.principal.subject}`
                : "No account identity"}
            </span>
          </footer>
        </div>
      </main>
    </div>
  );
}
