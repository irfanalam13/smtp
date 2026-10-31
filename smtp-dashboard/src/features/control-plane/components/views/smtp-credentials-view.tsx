import { useState } from "react";
import type { FormEvent } from "react";
import { ConfirmationDialog } from "@/components/ui/confirmation-dialog";
import { ServerIcon } from "@/components/ui/icons";
import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime } from "../../utils/formatters";
import type {
  CreatedSMTPCredential,
  SMTPCredential,
} from "../../types/smtp-credentials";

type SMTPCredentialsViewProps = {
  credentials: SMTPCredential[];
  loading: boolean;
  error: string;
  username: string;
  expiresAt: string;
  busyAction: string;
  oneTimeCredential: CreatedSMTPCredential | null;
  secretCopied: boolean;
  onUsernameChange: (value: string) => void;
  onExpiryChange: (value: string) => void;
  onCreate: (event: FormEvent<HTMLFormElement>) => void;
  onRotate: (credentialId: string) => void;
  onRevoke: (credentialId: string) => void;
  onCopy: () => void;
  onDismiss: () => void;
  onRetry: () => void;
  currentTime: number | null;
};

export function SMTPCredentialsView({
  credentials,
  loading,
  error,
  username,
  expiresAt,
  busyAction,
  oneTimeCredential,
  secretCopied,
  onUsernameChange,
  onExpiryChange,
  onCreate,
  onRotate,
  onRevoke,
  onCopy,
  onDismiss,
  onRetry,
  currentTime,
}: SMTPCredentialsViewProps) {
  const [credentialToRevoke, setCredentialToRevoke] =
    useState<SMTPCredential | null>(null);

  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading title="Create SMTP credentials" />
        <p className="panel-description">
          These credentials are for SMTP AUTH submission. The generated password
          is shown once and is not retrievable later. Requires a durable
          user-bound account and the smtp:credentials:write scope.
        </p>
        <form className="key-form" onSubmit={onCreate}>
          <label htmlFor="smtp-credential-username">Username</label>
          <input
            autoComplete="off"
            id="smtp-credential-username"
            maxLength={128}
            onChange={(event) => onUsernameChange(event.target.value)}
            pattern="[A-Za-z0-9._+@-]{1,128}"
            required
            value={username}
          />
          <label htmlFor="smtp-credential-expiry">Expires at</label>
          <input
            id="smtp-credential-expiry"
            onChange={(event) => onExpiryChange(event.target.value)}
            required
            type="datetime-local"
            value={expiresAt}
          />
          <button
            className="button button-primary"
            disabled={busyAction !== ""}
            type="submit"
          >
            {busyAction === "create" ? "Creating..." : "Create SMTP credential"}
          </button>
        </form>
      </section>

      {oneTimeCredential && (
        <section className="panel" aria-live="polite">
          <PanelHeading title="Save this password now" />
          <p>{oneTimeCredential.warning}</p>
          <dl>
            <div>
              <dt>SMTP username</dt>
              <dd>
                <code>{oneTimeCredential.authcid}</code>
              </dd>
            </div>
            <div>
              <dt>Password</dt>
              <dd>
                <code>{oneTimeCredential.password}</code>
              </dd>
            </div>
            <div>
              <dt>Expires</dt>
              <dd>{formatTime(oneTimeCredential.expires_at)}</dd>
            </div>
          </dl>
          <button
            className="button button-primary"
            onClick={onCopy}
            type="button"
          >
            {secretCopied ? "Copied" : "Copy username and password"}
          </button>{" "}
          <button
            className="button button-secondary"
            onClick={onDismiss}
            type="button"
          >
            Done
          </button>
        </section>
      )}

      <section className="panel full-width">
        <PanelHeading
          title="SMTP credentials"
          trailing={`${credentials.length} shown`}
        />
        {error ? (
          <InlineError message={error} onRetry={onRetry} />
        ) : loading ? (
          <EmptyState loading message="Loading SMTP credential metadata..." />
        ) : credentials.length === 0 ? (
          <EmptyState message="No SMTP credentials are available to this account." />
        ) : (
          <div className="key-list">
            {credentials.map((credential) => {
              const inactive =
                Boolean(credential.revoked_at) ||
                (currentTime !== null &&
                  Date.parse(credential.expires_at ?? "") <= currentTime);
              return (
                <article className="key-item" key={credential.id}>
                  <span className="key-symbol" aria-hidden="true">
                    <ServerIcon className="h-4 w-4" />
                  </span>
                  <span className="key-details">
                    <strong>{credential.username}</strong>
                    <small>Created {formatTime(credential.created_at)}</small>
                    <small>
                      {credential.revoked_at
                        ? `Revoked ${formatTime(credential.revoked_at)}`
                        : `Expires ${formatTime(credential.expires_at ?? "")}`}
                    </small>
                  </span>
                  <span
                    className={`status-badge ${inactive ? "revoked" : "active"}`}
                  >
                    {inactive ? "Inactive" : "Active"}
                  </span>
                  {!inactive && (
                    <div className="key-actions">
                      <button
                        className="button button-secondary"
                        disabled={busyAction !== ""}
                        onClick={() => onRotate(credential.id)}
                        type="button"
                      >
                        {busyAction === `rotate:${credential.id}`
                          ? "Rotating..."
                          : "Rotate"}
                      </button>
                      <button
                        className="button button-danger"
                        disabled={busyAction !== ""}
                        onClick={() => setCredentialToRevoke(credential)}
                        type="button"
                      >
                        Revoke
                      </button>
                    </div>
                  )}
                </article>
              );
            })}
          </div>
        )}
      </section>
      <ConfirmationDialog
        confirmLabel="Revoke credential"
        description={`SMTP AUTH will stop accepting the password for ${credentialToRevoke?.username ?? "this credential"}.`}
        onCancel={() => setCredentialToRevoke(null)}
        onConfirm={() => {
          if (!credentialToRevoke) return;
          onRevoke(credentialToRevoke.id);
          setCredentialToRevoke(null);
        }}
        open={Boolean(credentialToRevoke)}
        title="Revoke this SMTP credential?"
      />
    </div>
  );
}
