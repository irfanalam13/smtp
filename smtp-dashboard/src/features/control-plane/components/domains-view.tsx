import {
  EmptyState,
  InlineError,
  PanelHeading,
  statusLabel,
} from "./dashboard-primitives";
import type { FormEvent } from "react";
import type {
  Domain,
  DomainCheck,
  VerificationChallenge,
} from "../types/control-plane";

type DomainsViewProps = {
  domains: Domain[];
  domainInput: string;
  loading: boolean;
  error?: string;
  challenges: Record<string, VerificationChallenge>;
  domainChecks: Record<string, DomainCheck[]>;
  onDomainInputChange: (value: string) => void;
  onAddDomain: (event: FormEvent<HTMLFormElement>) => void;
  onCreateChallenge: (domainId: string) => void;
  onRunChecks: (domainId: string) => void;
  onConfirmOwnership: (domainId: string) => void;
};

export function DomainsView({
  domains,
  domainInput,
  loading,
  error,
  challenges,
  domainChecks,
  onDomainInputChange,
  onAddDomain,
  onCreateChallenge,
  onRunChecks,
  onConfirmOwnership,
}: DomainsViewProps) {
  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading title="Register a domain" />
        <p className="panel-description">
          Verify ownership before using a domain as a sender address.
        </p>
        <form className="inline-form" onSubmit={onAddDomain}>
          <label className="visually-hidden" htmlFor="domain-name">
            Domain name
          </label>
          <input
            autoCapitalize="none"
            id="domain-name"
            onChange={(event) => onDomainInputChange(event.target.value)}
            placeholder="example.com"
            required
            value={domainInput}
          />
          <button className="button button-primary" type="submit">
            Add domain
          </button>
        </form>
      </section>
      <section className="panel full-width">
        <PanelHeading
          title="Your domains"
          trailing={`${domains.length} total`}
        />
        {error ? (
          <InlineError message={error} />
        ) : loading ? (
          <EmptyState loading message="Loading domains..." />
        ) : domains.length === 0 ? (
          <EmptyState message="No sending domains have been registered yet." />
        ) : (
          <div className="domain-list">
            {domains.map((item) => (
              <article className="domain-item" key={item.id}>
                <div className="domain-main">
                  <span className="domain-symbol" aria-hidden="true">
                    @
                  </span>
                  <span>
                    <strong>{item.domain}</strong>
                    <small>Sending domain</small>
                  </span>
                </div>
                <span className={`status-badge ${item.status}`}>
                  <span className="badge-dot" />
                  {statusLabel(item.status)}
                </span>
                <div className="domain-actions">
                  <button
                    className="button button-secondary"
                    onClick={() => onCreateChallenge(item.id)}
                    type="button"
                  >
                    DNS record
                  </button>
                  <button
                    className="button button-quiet"
                    onClick={() => onRunChecks(item.id)}
                    type="button"
                  >
                    Run checks
                  </button>
                  {challenges[item.id] && (
                    <button
                      className="button button-quiet"
                      onClick={() => onConfirmOwnership(item.id)}
                      type="button"
                    >
                      Confirm ownership
                    </button>
                  )}
                </div>
                {challenges[item.id] && (
                  <div className="challenge-box">
                    <strong>Publish this DNS TXT record</strong>
                    <code>
                      {challenges[item.id].record_name}{" "}
                      {challenges[item.id].record_type}{" "}
                      {challenges[item.id].record_value}
                    </code>
                    <small>
                      Keep this challenge secret. It is shown only in this
                      browser session.
                    </small>
                  </div>
                )}
                {domainChecks[item.id]?.length ? (
                  <div className="check-results">
                    {domainChecks[item.id].map((check) => (
                      <div className="check-result" key={check.kind}>
                        <span>
                          <strong>{check.kind}</strong>
                          <small>{check.explanation}</small>
                          {check.status !== "pass" && (
                            <small>{check.remediation}</small>
                          )}
                        </span>
                        <span className={`status-badge ${check.status}`}>
                          {check.status}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : null}
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
