import { useState } from "react";
import type { FormEvent } from "react";
import { ConfirmationDialog } from "@/components/ui/confirmation-dialog";
import { CheckIcon, CopyIcon, GlobeIcon } from "@/components/ui/icons";
import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime, statusLabel } from "../../utils/formatters";
import type {
  Domain,
  DomainCheck,
  DomainReadiness,
  VerificationChallenge,
} from "../../types/control-plane";

type DomainsViewProps = {
  domains: Domain[];
  domainInput: string;
  domainInputError: string;
  loading: boolean;
  error?: string;
  challenges: Record<string, VerificationChallenge>;
  domainChecks: Record<string, DomainCheck>;
  domainReadiness: Record<string, DomainReadiness>;
  domainDetailsLoading: Record<string, boolean>;
  domainDetailsErrors: Record<string, string>;
  busyAction: string;
  onDomainInputChange: (value: string) => void;
  onAddDomain: (event: FormEvent<HTMLFormElement>) => void;
  onCreateChallenge: (domainId: string) => void;
  onRunChecks: (domainId: string) => void;
  onConfirmOwnership: (domainId: string) => void;
  onDisableDomain: (domain: Domain) => void;
  onLoadDomainDetails: (domainId: string) => void;
  onRetry: () => void;
};

export function DomainsView({
  domains,
  domainInput,
  domainInputError,
  loading,
  error,
  challenges,
  domainChecks,
  domainReadiness,
  domainDetailsLoading,
  domainDetailsErrors,
  busyAction,
  onDomainInputChange,
  onAddDomain,
  onCreateChallenge,
  onRunChecks,
  onConfirmOwnership,
  onDisableDomain,
  onLoadDomainDetails,
  onRetry,
}: DomainsViewProps) {
  const [domainToDisable, setDomainToDisable] = useState<Domain | null>(null);
  const [copiedDomainId, setCopiedDomainId] = useState("");

  async function copyChallenge(domainId: string, value: string) {
    try {
      await navigator.clipboard.writeText(value);
      setCopiedDomainId(domainId);
    } catch {
      setCopiedDomainId("");
    }
  }

  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading title="Register a domain" />
        <p className="panel-description">
          Verify ownership before using this domain as a sender address.
          Registration and DNS checks use the authenticated control-plane API.
        </p>
        <form className="inline-form" onSubmit={onAddDomain}>
          <label className="visually-hidden" htmlFor="domain-name">
            Domain name
          </label>
          <input
            aria-describedby={
              domainInputError ? "domain-name-error" : undefined
            }
            aria-invalid={Boolean(domainInputError)}
            autoCapitalize="none"
            autoComplete="url"
            id="domain-name"
            maxLength={253}
            onChange={(event) => onDomainInputChange(event.target.value)}
            placeholder="example.com"
            required
            value={domainInput}
          />
          <button
            className="button button-primary"
            disabled={busyAction === "domain:add"}
            type="submit"
          >
            {busyAction === "domain:add" ? "Registering..." : "Add domain"}
          </button>
        </form>
        {domainInputError && (
          <p className="field-error" id="domain-name-error" role="alert">
            {domainInputError}
          </p>
        )}
      </section>

      <section className="panel full-width">
        <PanelHeading
          title="Your domains"
          trailing={`${domains.length} shown`}
        />
        {error ? (
          <InlineError message={error} onRetry={onRetry} />
        ) : loading ? (
          <EmptyState loading message="Loading domains..." />
        ) : domains.length === 0 ? (
          <EmptyState message="No sending domains have been registered yet." />
        ) : (
          <div className="domain-list">
            {domains.map((item) => {
              const check = domainChecks[item.id];
              const readiness = domainReadiness[item.id];
              return (
                <article className="domain-item" key={item.id}>
                  <div className="domain-main">
                    <span className="domain-symbol" aria-hidden="true">
                      <GlobeIcon className="h-4 w-4" />
                    </span>
                    <span>
                      <strong>{item.domain}</strong>
                      <small>Created {formatTime(item.created_at)}</small>
                    </span>
                  </div>
                  <span className={`status-badge ${item.status}`}>
                    <span className="badge-dot" />
                    {statusLabel(item.status)}
                  </span>
                  <div className="domain-actions">
                    <button
                      className="button button-secondary"
                      disabled={busyAction === `domain:challenge:${item.id}`}
                      onClick={() => onCreateChallenge(item.id)}
                      type="button"
                    >
                      {busyAction === `domain:challenge:${item.id}`
                        ? "Creating..."
                        : "DNS record"}
                    </button>
                    <button
                      className="button button-quiet"
                      disabled={busyAction === `domain:checks:${item.id}`}
                      onClick={() => onRunChecks(item.id)}
                      type="button"
                    >
                      {busyAction === `domain:checks:${item.id}`
                        ? "Checking..."
                        : "Run checks"}
                    </button>
                    <button
                      className="button button-quiet"
                      disabled={domainDetailsLoading[item.id]}
                      onClick={() => onLoadDomainDetails(item.id)}
                      type="button"
                    >
                      {domainDetailsLoading[item.id]
                        ? "Loading..."
                        : "Refresh status"}
                    </button>
                    {challenges[item.id] && (
                      <button
                        className="button button-quiet"
                        disabled={busyAction === `domain:verify:${item.id}`}
                        onClick={() => onConfirmOwnership(item.id)}
                        type="button"
                      >
                        {busyAction === `domain:verify:${item.id}`
                          ? "Verifying..."
                          : "Check verification"}
                      </button>
                    )}
                    {item.status !== "disabled" && (
                      <button
                        className="button button-danger-quiet"
                        disabled={busyAction === `domain:disable:${item.id}`}
                        onClick={() => setDomainToDisable(item)}
                        type="button"
                      >
                        {busyAction === `domain:disable:${item.id}`
                          ? "Disabling..."
                          : "Disable"}
                      </button>
                    )}
                  </div>

                  {challenges[item.id] && (
                    <div className="challenge-box">
                      <div className="challenge-heading">
                        <strong>Publish this DNS TXT record</strong>
                        <button
                          className="copy-button"
                          onClick={() =>
                            void copyChallenge(
                              item.id,
                              `${challenges[item.id].record_name} ${challenges[item.id].record_type} ${challenges[item.id].record_value}`,
                            )
                          }
                          type="button"
                        >
                          {copiedDomainId === item.id ? (
                            <CheckIcon aria-hidden="true" />
                          ) : (
                            <CopyIcon aria-hidden="true" />
                          )}
                          {copiedDomainId === item.id ? "Copied" : "Copy"}
                        </button>
                      </div>
                      <code>
                        {challenges[item.id].record_name}{" "}
                        {challenges[item.id].record_type}{" "}
                        {challenges[item.id].record_value}
                      </code>
                      <small>
                        Keep this verification record within your DNS
                        configuration; the challenge is held only in this
                        browser session.
                      </small>
                    </div>
                  )}

                  {readiness && (
                    <div className="domain-readiness">
                      <span>
                        <strong>Readiness</strong>
                        <small>
                          {readiness.checked_at
                            ? `Checked ${formatTime(readiness.checked_at)}`
                            : "Run DNS checks to evaluate readiness."}
                        </small>
                      </span>
                      <span className={`status-badge ${readiness.status}`}>
                        {statusLabel(readiness.status)}
                      </span>
                    </div>
                  )}
                  {domainDetailsErrors[item.id] && (
                    <InlineError
                      message={domainDetailsErrors[item.id]}
                      onRetry={() => onLoadDomainDetails(item.id)}
                    />
                  )}
                  {domainDetailsLoading[item.id] && (
                    <EmptyState loading message="Loading DNS status..." />
                  )}
                  {check && (
                    <div className="check-results">
                      <div className="diagnostic-heading">
                        <strong>DNS diagnostics</strong>
                        <small>
                          {check.stale
                            ? "No current cached result"
                            : `Checked ${formatTime(check.checked_at)}`}
                        </small>
                      </div>
                      {check.results.map((result) => (
                        <div className="check-result" key={result.kind}>
                          <span>
                            <strong>{statusLabel(result.kind)}</strong>
                            <small>{result.explanation}</small>
                            {result.status !== "pass" && (
                              <small>{result.remediation}</small>
                            )}
                          </span>
                          <span className={`status-badge ${result.status}`}>
                            {statusLabel(result.status)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </article>
              );
            })}
          </div>
        )}
      </section>
      <ConfirmationDialog
        confirmLabel="Disable domain"
        description={`New submissions from ${domainToDisable?.domain ?? "this domain"} will be rejected. Existing records remain visible.`}
        onCancel={() => setDomainToDisable(null)}
        onConfirm={() => {
          if (!domainToDisable) return;
          onDisableDomain(domainToDisable);
          setDomainToDisable(null);
        }}
        open={Boolean(domainToDisable)}
        title="Disable this sending domain?"
      />
    </div>
  );
}
