import {
  ActivityIcon,
  GlobeIcon,
  KeyIcon,
  ServerIcon,
} from "@/components/ui/icons";
import { PanelHeading } from "../shared/panel-heading";
import { StatCard } from "../shared/stat-card";
import { SubmissionTable } from "../shared/submission-table";
import type {
  ApiKey,
  Domain,
  LoadErrors,
  Principal,
  Submission,
  Tenant,
  Usage,
} from "../../types/control-plane";

type OverviewViewProps = {
  domains: Domain[];
  verifiedDomains: number;
  keys: ApiKey[];
  submissions: Submission[];
  usage: Usage | null;
  principal: Principal | null;
  tenant: Tenant | null;
  readiness: "ready" | "not_ready" | null;
  currentTime: number | null;
  loading: boolean;
  loadErrors: LoadErrors;
  onRetry: () => void;
  onCompose: () => void;
  onActivity: () => void;
  onDomains: () => void;
  onApiKeys: () => void;
};

export function OverviewView({
  domains,
  verifiedDomains,
  keys,
  submissions,
  usage,
  principal,
  tenant,
  readiness,
  currentTime,
  loading,
  loadErrors,
  onRetry,
  onCompose,
  onActivity,
  onDomains,
  onApiKeys,
}: OverviewViewProps) {
  const hasVerifiedDomain = verifiedDomains > 0;
  const hasSendingKey = keys.some(
    (key) =>
      !key.revoked_at &&
      (!key.expires_at ||
        (currentTime !== null && Date.parse(key.expires_at) > currentTime)) &&
      key.scopes.includes("delivery:write"),
  );
  const hasSubmittedEmail = submissions.length > 0;
  const setupComplete =
    Number(hasVerifiedDomain) +
    Number(hasSendingKey) +
    Number(hasSubmittedEmail);

  return (
    <>
      <section aria-labelledby="overview-heading" className="relative isolate overflow-hidden rounded-3xl border border-emerald-100 bg-gradient-to-br from-emerald-50 via-white to-teal-50 px-6 py-8 text-zinc-900 shadow-sm sm:px-10 sm:py-10">
        <div className="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-emerald-200/40 blur-3xl" />
        <div className="relative max-w-2xl">
          <span className="inline-flex rounded-full border border-emerald-200 bg-white/70 px-3 py-1 text-[10px] font-bold tracking-[0.18em] text-emerald-700">WORKSPACE STATUS</span>
          <h2 id="overview-heading" className="mt-5 max-w-xl text-3xl font-semibold tracking-tight text-emerald-950 sm:text-4xl">Everything you need to send with confidence.</h2>
          <p className="mt-4 max-w-xl text-sm leading-6 text-zinc-600 sm:text-base">
            Verify your domain, issue only the credentials you need, and follow
            each message through the queue.
          </p>
          <div className="mt-7 flex flex-wrap gap-3">
            <button
              className="inline-flex items-center gap-2 rounded-xl bg-emerald-700 px-4 py-2.5 text-sm font-semibold text-white shadow-md shadow-emerald-700/20 transition hover:bg-emerald-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-700"
              onClick={onCompose}
              type="button"
            >
              Send an email <span aria-hidden="true">→</span>
            </button>
            <button
              className="inline-flex items-center rounded-xl border border-emerald-200 bg-white/70 px-4 py-2.5 text-sm font-semibold text-emerald-800 transition hover:bg-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-700"
              onClick={onActivity}
              type="button"
            >
              View activity
            </button>
          </div>
        </div>
        <div className="absolute bottom-6 right-6 hidden max-w-60 items-center gap-3 rounded-2xl border border-emerald-100 bg-white/80 px-4 py-3 shadow-sm backdrop-blur sm:flex">
          <span className="grid h-8 w-8 shrink-0 place-items-center rounded-xl bg-amber-100" aria-hidden="true">
            <span className="h-2 w-2 rounded-full bg-amber-500" />
          </span>
          <span className="min-w-0">
            <strong className="block text-xs font-semibold text-emerald-950">
              {readiness === "ready"
                ? "API ready"
                : loadErrors.readiness
                  ? "API unavailable"
                  : readiness === "not_ready"
                    ? "API not ready"
                    : "Checking API"}
            </strong>
            <small className="mt-1 block truncate text-[11px] leading-5 text-zinc-500">
              {loadErrors.readiness ??
                loadErrors.principal ??
                `${tenant?.name ?? principal?.tenant?.name ?? "Tenant details are not available"}${principal?.subject ? ` · ${principal.subject}` : ""}`}
            </small>
          </span>
        </div>
      </section>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="Usage this window"
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
          icon={<ServerIcon />}
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
          icon={<ActivityIcon />}
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
          icon={<GlobeIcon />}
        />
        <StatCard
          label="Active send keys"
          value={
            loading || loadErrors.keys
              ? "—"
              : keys
                  .filter(
                    (key) =>
                      !key.revoked_at &&
                      (!key.expires_at ||
                        (currentTime !== null &&
                          Date.parse(key.expires_at) > currentTime)) &&
                      key.scopes.includes("delivery:write"),
                  )
                  .length.toString()
          }
          detail={
            loadErrors.keys
              ? "Key data is unavailable"
              : "With delivery:write access"
          }
          tone="amber"
          icon={<KeyIcon />}
        />
      </div>

      <div className="mt-6 grid gap-6 xl:grid-cols-[minmax(0,1.45fr)_minmax(360px,0.8fr)]">
        <section className="rounded-2xl border border-zinc-200/80 bg-white p-5 shadow-sm sm:p-6">
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
            onRetry={onRetry}
          />
          <p className="home-honesty-note">
            Queue state does not confirm remote delivery or inbox placement.
          </p>
        </section>

        <section className="rounded-2xl border border-zinc-200/80 bg-white p-5 shadow-sm sm:p-6">
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
