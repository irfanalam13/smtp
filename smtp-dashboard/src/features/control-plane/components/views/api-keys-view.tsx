import { useState } from "react";
import type { FormEvent } from "react";
import { ConfirmationDialog } from "@/components/ui/confirmation-dialog";
import { KeyIcon } from "@/components/ui/icons";
import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime } from "../../utils/formatters";
import type { APIKeyScope, ApiKey } from "../../types/control-plane";

const availableScopes: { scope: APIKeyScope; description: string }[] = [
  { scope: "domains:read", description: "Read domain configuration" },
  { scope: "domains:write", description: "Register and verify domains" },
  { scope: "delivery:read", description: "Read usage and submissions" },
  { scope: "delivery:write", description: "Submit email" },
  { scope: "keys:write", description: "List, create, and revoke API keys" },
  {
    scope: "smtp:credentials:read",
    description: "Read SMTP credential metadata",
  },
  {
    scope: "smtp:credentials:write",
    description: "Manage SMTP credentials",
  },
  { scope: "mailbox:read", description: "Read permitted mailbox data" },
  { scope: "mailbox:write", description: "Modify mailbox messages" },
  { scope: "mailbox:manage", description: "Manage mailboxes and access" },
];

function isExpired(key: ApiKey, currentTime: number | null): boolean {
  return Boolean(
    key.expires_at &&
      currentTime !== null &&
      Date.parse(key.expires_at) <= currentTime,
  );
}

type ApiKeysViewProps = {
  keys: ApiKey[];
  keyName: string;
  keyNameError: string;
  keyScopes: APIKeyScope[];
  keyExpiresAt: string;
  keyExpiryPreset: string;
  oneTimeSecret: string;
  secretWarning: string;
  secretCopied: boolean;
  loading: boolean;
  error?: string;
  busyAction: string;
  currentTime: number | null;
  onKeyNameChange: (value: string) => void;
  onKeyExpiresAtChange: (value: string) => void;
  onKeyExpiryPresetChange: (value: string) => void;
  onToggleScope: (scope: APIKeyScope, selected: boolean) => void;
  onCreateKey: (event: FormEvent<HTMLFormElement>) => void;
  onCopySecret: () => void;
  onDismissSecret: () => void;
  onRevokeKey: (keyId: string) => void;
  onRetry: () => void;
};

export function ApiKeysView({
  keys,
  keyName,
  keyNameError,
  keyScopes,
  keyExpiresAt,
  keyExpiryPreset,
  oneTimeSecret,
  secretWarning,
  secretCopied,
  loading,
  error,
  busyAction,
  currentTime,
  onKeyNameChange,
  onKeyExpiresAtChange,
  onKeyExpiryPresetChange,
  onToggleScope,
  onCreateKey,
  onCopySecret,
  onDismissSecret,
  onRevokeKey,
  onRetry,
}: ApiKeysViewProps) {
  const [keyToRevoke, setKeyToRevoke] = useState<ApiKey | null>(null);

  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading
          title={oneTimeSecret ? "Copy your new API key" : "Create an API key"}
        />
        {oneTimeSecret ? (
          <div className="secret-panel" role="status" aria-live="polite">
            <div>
              <strong>Copy this API key now</strong>
              <p>{secretWarning}</p>
              <code>{oneTimeSecret}</code>
            </div>
            <div className="secret-actions">
              <button
                className="button button-primary"
                onClick={onCopySecret}
                type="button"
              >
                {secretCopied ? "Copied" : "Copy API key"}
              </button>
              <button
                className="button button-secondary"
                onClick={onDismissSecret}
                type="button"
              >
                Done
              </button>
            </div>
          </div>
        ) : (
          <>
            <p className="panel-description">
              Choose only the scopes the integration needs. The API returns the
              secret once; it is not stored by this dashboard.
            </p>
            <form className="key-form" onSubmit={onCreateKey}>
              <label htmlFor="key-name">Key name</label>
              <input
                aria-describedby={keyNameError ? "key-name-error" : undefined}
                aria-invalid={Boolean(keyNameError)}
                autoComplete="off"
                id="key-name"
                maxLength={120}
                onChange={(event) => onKeyNameChange(event.target.value)}
                placeholder="Production mailer"
                required
                value={keyName}
              />
              {keyNameError && (
                <p className="field-error" id="key-name-error" role="alert">
                  {keyNameError}
                </p>
              )}
              <label htmlFor="key-expiry">Key lifetime</label>
              <select
                id="key-expiry"
                onChange={(event) => onKeyExpiryPresetChange(event.target.value)}
                value={keyExpiryPreset}
              >
                <option value="1_week">1 week</option>
                <option value="1_month">1 month</option>
                <option value="3_months">3 months</option>
                <option value="6_months">6 months</option>
                <option value="1_year">1 year</option>
                <option value="">Never expires</option>
                <option value="custom">Custom date</option>
              </select>
              {keyExpiryPreset === "custom" && (
                <input
                  id="key-expires-at"
                  aria-label="Custom expiration date"
                  onChange={(event) => onKeyExpiresAtChange(event.target.value)}
                  type="datetime-local"
                  value={keyExpiresAt}
                />
              )}
              <fieldset className="scope-options">
                <legend>Permissions</legend>
                {availableScopes.map(({ scope, description }) => (
                  <label className="scope-option" key={scope}>
                    <input
                      checked={keyScopes.includes(scope)}
                      onChange={(event) =>
                        onToggleScope(scope, event.target.checked)
                      }
                      type="checkbox"
                    />
                    <span>
                      <code>{scope}</code>
                      <small>{description}</small>
                    </span>
                  </label>
                ))}
              </fieldset>
              <button
                className="button button-primary"
                disabled={
                  busyAction === "key:create" ||
                  keyScopes.length === 0 ||
                  keyScopes.length > 10
                }
                type="submit"
              >
                {busyAction === "key:create" ? "Creating..." : "Create key"}
              </button>
            </form>
          </>
        )}
      </section>

      <section className="panel full-width">
        <PanelHeading
          title="Your API keys"
          trailing="Latest 50 returned by the API"
        />
        {error ? (
          <InlineError message={error} onRetry={onRetry} />
        ) : loading ? (
          <EmptyState loading message="Loading API keys..." />
        ) : keys.length === 0 ? (
          <EmptyState message="No API keys are available to this account." />
        ) : (
          <div className="key-list">
            {keys.map((item) => {
              const expired = isExpired(item, currentTime);
              const revoked = Boolean(item.revoked_at);
              const status = revoked
                ? "Revoked"
                : item.expires_at && currentTime === null
                  ? "Checking"
                  : expired
                    ? "Expired"
                    : "Active";
              return (
                <div className="key-item" key={item.id}>
                  <span className="key-symbol" aria-hidden="true">
                    <KeyIcon className="h-4 w-4" />
                  </span>
                  <span className="key-details">
                    <strong>{item.name}</strong>
                    <small>
                      {item.prefix} · {item.scopes.join(", ")}
                    </small>
                    <small>
                      Created {formatTime(item.created_at)}
                      {item.expires_at
                        ? ` · Expires ${formatTime(item.expires_at)}`
                        : " · No expiry"}
                    </small>
                  </span>
                  <span
                    className={`status-badge ${status.toLowerCase()}`}
                    aria-label={`Key status: ${status}`}
                  >
                    {status}
                  </span>
                  {!revoked && !expired && status !== "Checking" && (
                    <button
                      aria-label={`Revoke ${item.name}`}
                      className="button button-danger-quiet"
                      disabled={busyAction === `key:revoke:${item.id}`}
                      onClick={() => setKeyToRevoke(item)}
                      type="button"
                    >
                      {busyAction === `key:revoke:${item.id}`
                        ? "Revoking..."
                        : "Revoke"}
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>
      <ConfirmationDialog
        confirmLabel="Revoke key"
        description={`Integrations using ${keyToRevoke?.name ?? "this key"} will stop authenticating immediately.`}
        onCancel={() => setKeyToRevoke(null)}
        onConfirm={() => {
          if (!keyToRevoke) return;
          onRevokeKey(keyToRevoke.id);
          setKeyToRevoke(null);
        }}
        open={Boolean(keyToRevoke)}
        title="Revoke this API key?"
      />
    </div>
  );
}
