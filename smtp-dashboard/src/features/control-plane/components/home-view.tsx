import {
  PanelHeading,
  StatCard,
  SubmissionTable,
} from "./dashboard-primitives";
import type {
  ApiKey,
  Domain,
  LoadErrors,
  Submission,
  Usage,
} from "../types/control-plane";

type HomeViewProps = {
  domains: Domain[];
  verifiedDomains: number;
  keys: ApiKey[];
  submissions: Submission[];
  usage: Usage | null;
  loading: boolean;
  loadErrors: LoadErrors;
  onCompose: () => void;
  onActivity: () => void;
  onDomains: () => void;
  onApiKeys: () => void;
};

export function HomeView({
  domains,
  verifiedDomains,
  keys,
  submissions,
  usage,
  loading,
  loadErrors,
  onCompose,
  onActivity,
  onDomains,
  onApiKeys,
}: HomeViewProps) {
  const hasVerifiedDomain = verifiedDomains > 0;
  const hasSendingKey = keys.some(
    (key) => !key.revoked_at && key.scopes.includes("delivery:write"),
  );
  const hasSubmittedEmail = submissions.length > 0;
  const setupComplete =
    Number(hasVerifiedDomain) +
    Number(hasSendingKey) +
    Number(hasSubmittedEmail);

  return (
    <>
      <section className="home-hero">
        <div className="home-hero-copy">
          <span className="home-hero-kicker">EMAIL WORKSPACE</span>
          <h2>Everything starts with a good send.</h2>
          <p>
            Manage your sending setup, submit a test message, and follow its
            queue activity from one place.
          </p>
          <div className="home-hero-actions">
            <button
              className="button button-home-primary"
              onClick={onCompose}
              type="button"
            >
              Send an email <span aria-hidden="true">→</span>
            </button>
            <button
              className="button button-home-secondary"
              onClick={onActivity}
              type="button"
            >
              View activity
            </button>
          </div>
        </div>
        <div className="home-hero-status">
          <span className="home-status-mark" aria-hidden="true">
            <span />
          </span>
          <span>
            <strong>Development environment</strong>
            <small>
              Queue outcomes are simulated; no email is sent externally.
            </small>
          </span>
        </div>
      </section>

      <div className="stat-grid">
        <StatCard
          label="Submissions"
          value={
            loading || loadErrors.usage
              ? "—"
              : (usage?.messages_used.toLocaleString() ?? "—")
          }
          detail={
            loadErrors.usage
              ? "Usage is unavailable"
              : usage
                ? `of ${usage.message_limit.toLocaleString()} in this window`
                : "No usage data available"
          }
          tone="blue"
          icon="↗"
        />
        <StatCard
          label="Recent messages"
          value={
            loading || loadErrors.submissions
              ? "—"
              : submissions.length.toString()
          }
          detail={
            loadErrors.submissions
              ? "History is unavailable"
              : "Latest 20 submissions"
          }
          tone="green"
          icon="EM"
        />
        <StatCard
          label="Verified domains"
          value={
            loading || loadErrors.domains ? "—" : verifiedDomains.toString()
          }
          detail={
            loadErrors.domains
              ? "Domain data is unavailable"
              : `of ${domains.length} sending domains`
          }
          tone="violet"
          icon="DM"
        />
        <StatCard
          label="Active send keys"
          value={
            loading || loadErrors.keys
              ? "—"
              : keys
                  .filter(
                    (key) =>
                      !key.revoked_at && key.scopes.includes("delivery:write"),
                  )
                  .length.toString()
          }
          detail={
            loadErrors.keys
              ? "Key data is unavailable"
              : "With delivery:write access"
          }
          tone="amber"
          icon="KEY"
        />
      </div>

      <div className="home-content-grid">
        <section className="panel home-activity-panel">
          <PanelHeading
            title="Recent email activity"
            action={
              <button
                className="text-button"
                onClick={onActivity}
                type="button"
              >
                View all <span aria-hidden="true">→</span>
              </button>
            }
          />
          <SubmissionTable
            items={submissions.slice(0, 5)}
            error={loadErrors.submissions}
            loading={loading}
          />
          <p className="home-honesty-note">
            “Queued” confirms acceptance only. It does not confirm delivery or
            inbox placement.
          </p>
        </section>

        <section className="panel home-setup-panel">
          <PanelHeading
            title="Get your first email flowing"
            trailing={
              loading ? "Checking setup" : `${setupComplete}/3 complete`
            }
          />
          <p className="home-setup-intro">
            Complete these steps to prepare your account for sending.
          </p>
          <div
            aria-label={`${setupComplete} of 3 setup steps complete`}
            className="setup-progress"
            role="progressbar"
            aria-valuemax={3}
            aria-valuemin={0}
            aria-valuenow={setupComplete}
          >
            <span style={{ width: `${(setupComplete / 3) * 100}%` }} />
          </div>
          <ol className="setup-list">
            <li className={hasVerifiedDomain ? "complete" : ""}>
              <span className="setup-check">
                {hasVerifiedDomain ? "✓" : "1"}
              </span>
              <span className="setup-copy">
                <strong>Verify a sending domain</strong>
                <small>
                  {loadErrors.domains
                    ? "Domain status could not be checked."
                    : hasVerifiedDomain
                      ? `${verifiedDomains} verified and ready for setup.`
                      : "Add a domain and publish its verification record."}
                </small>
              </span>
              {!hasVerifiedDomain && !loadErrors.domains && (
                <button
                  className="setup-action"
                  onClick={onDomains}
                  type="button"
                >
                  Set up
                </button>
              )}
            </li>
            <li className={hasSendingKey ? "complete" : ""}>
              <span className="setup-check">{hasSendingKey ? "✓" : "2"}</span>
              <span className="setup-copy">
                <strong>Create a sending API key</strong>
                <small>
                  {loadErrors.keys
                    ? "API key status could not be checked."
                    : hasSendingKey
                      ? "A key with delivery:write access is available."
                      : "Create a scoped key for your sending integration."}
                </small>
              </span>
              {!hasSendingKey && !loadErrors.keys && (
                <button
                  className="setup-action"
                  onClick={onApiKeys}
                  type="button"
                >
                  Set up
                </button>
              )}
            </li>
            <li className={hasSubmittedEmail ? "complete" : ""}>
              <span className="setup-check">
                {hasSubmittedEmail ? "✓" : "3"}
              </span>
              <span className="setup-copy">
                <strong>Submit your first email</strong>
                <small>
                  {loadErrors.submissions
                    ? "Submission history could not be checked."
                    : hasSubmittedEmail
                      ? "Your first message has been accepted into the queue."
                      : "Send a test message and inspect its queue activity."}
                </small>
              </span>
              {!hasSubmittedEmail && !loadErrors.submissions && (
                <button
                  className="setup-action"
                  onClick={onCompose}
                  type="button"
                >
                  Compose
                </button>
              )}
            </li>
          </ol>
          {usage && (
            <p className="home-setup-footnote">
              {Math.round(usage.max_message_bytes / 1024 / 1024)} MB max message
              size. Usage counters are currently process-local.
            </p>
          )}
        </section>
      </div>
    </>
  );
}
