"use client";

import { useControlPlaneDashboard } from "../hooks/use-control-plane-dashboard";
import { ApiKeysView } from "./api-keys-view";
import { ActivityView } from "./activity-view";
import { DashboardSidebar } from "./dashboard-sidebar";
import { DomainsView } from "./domains-view";
import { MailboxesView } from "./mailboxes-view";
import { HomeView } from "./home-view";
import { SendEmailView } from "./send-email-view";
import type { DashboardView } from "../types/control-plane";

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
  "api-keys": {
    title: "API keys",
    description: "Create and revoke scoped credentials.",
  },
  credentials: {
    title: "Credentials",
    description: "Connect with an API key or OIDC access token.",
  },
};

export default function ControlPlaneDashboard() {
  const dashboard = useControlPlaneDashboard();
  const currentTitle = viewTitles[dashboard.view];

  return (
    <div className="app-shell">
      <DashboardSidebar
        key={dashboard.view}
        brandName={brandName}
        view={dashboard.view}
        onViewChange={dashboard.setView}
        connected={dashboard.connected}
        loading={dashboard.loading}
        connectionError={dashboard.connectionError}
        principal={dashboard.principal}
      />

      <main className="main-panel">
        <header className="topbar">
          <div className="breadcrumb">
            <span>Workspace</span>
            <span className="breadcrumb-separator">/</span>
            <strong>{currentTitle.title}</strong>
          </div>
          <div className="topbar-right">
            <span
              className={`api-pill ${dashboard.connected ? "connected" : ""}`}
            >
              <span className="connection-dot" />
              {dashboard.connected
                ? "Connected"
                : dashboard.loading
                  ? "Connecting"
                  : "Offline"}
            </span>
            <span className="avatar top-avatar">
              {(dashboard.principal?.subject?.[0] ?? "U").toUpperCase()}
            </span>
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
                        ? "Using local development identity"
                        : "Connect your account"}
                  </strong>
                  <small>
                    {dashboard.apiToken
                      ? "The token is held in memory only and cleared when the page reloads."
                      : localDevelopment
                        ? "Use the temporary local identity, or paste a durable user-bound key."
                        : "Paste a user-bound API key or OIDC access token to connect."}
                    {dashboard.principalError && dashboard.connected
                      ? ` ${dashboard.principalError}`
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
            </section>
          )}

          {dashboard.view !== "home" &&
            dashboard.view !== "credentials" && (
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

          {dashboard.view === "home" && (
            <HomeView
              domains={dashboard.domains}
              verifiedDomains={dashboard.verifiedDomains}
              keys={dashboard.keys}
              submissions={dashboard.submissions}
              usage={dashboard.usage}
              loading={dashboard.loading}
              loadErrors={dashboard.loadErrors}
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
              subject={dashboard.subject}
              body={dashboard.body}
              sending={dashboard.sending}
              loading={dashboard.loading}
              submissions={dashboard.submissions}
              loadErrors={dashboard.loadErrors}
              onSenderChange={(value) => {
                dashboard.setSender(value);
                dashboard.setSubmissionIdempotencyKey("");
              }}
              onRecipientsChange={(value) => {
                dashboard.setRecipients(value);
                dashboard.setSubmissionIdempotencyKey("");
              }}
              onSubjectChange={(value) => {
                dashboard.setSubject(value);
                dashboard.setSubmissionIdempotencyKey("");
              }}
              onBodyChange={(value) => {
                dashboard.setBody(value);
                dashboard.setSubmissionIdempotencyKey("");
              }}
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
              error={dashboard.activityError}
              onSelectSubmission={(submissionId) =>
                void dashboard.loadSubmissionAttempts(submissionId)
              }
            />
          )}
          {dashboard.view === "domains" && (
            <DomainsView
              domains={dashboard.domains}
              domainInput={dashboard.domainInput}
              loading={dashboard.loading}
              error={dashboard.loadErrors.domains}
              challenges={dashboard.challenges}
              domainChecks={dashboard.domainChecks}
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
            />
          )}
          {dashboard.view === "mailboxes" && (
            <MailboxesView
              mailboxes={dashboard.mailboxes}
              messages={dashboard.messages}
              selectedMailbox={dashboard.selectedMailbox}
              loading={dashboard.loading}
              error={dashboard.loadErrors.mailboxes}
              onSelectMailbox={(mailboxId) =>
                void dashboard.selectMailbox(mailboxId)
              }
            />
          )}
          {dashboard.view === "api-keys" && (
            <ApiKeysView
              keys={dashboard.keys}
              keyName={dashboard.keyName}
              oneTimeSecret={dashboard.oneTimeSecret}
              secretCopied={dashboard.secretCopied}
              loading={dashboard.loading}
              error={dashboard.loadErrors.keys}
              onKeyNameChange={dashboard.setKeyName}
              onCreateKey={dashboard.addApiKey}
              onCopySecret={() => void dashboard.copyApiKey()}
              onDismissSecret={dashboard.dismissApiKeySecret}
              onRevokeKey={(keyId) => void dashboard.removeApiKey(keyId)}
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
