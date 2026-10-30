import { EmptyState, InlineError, PanelHeading } from "./dashboard-primitives";
import type { FormEvent } from "react";
import type { ApiKey } from "../types/control-plane";

type ApiKeysViewProps = {
  keys: ApiKey[];
  keyName: string;
  oneTimeSecret: string;
  secretCopied: boolean;
  loading: boolean;
  error?: string;
  onKeyNameChange: (value: string) => void;
  onCreateKey: (event: FormEvent<HTMLFormElement>) => void;
  onCopySecret: () => void;
  onDismissSecret: () => void;
  onRevokeKey: (keyId: string) => void;
};

export function ApiKeysView({
  keys,
  keyName,
  oneTimeSecret,
  secretCopied,
  loading,
  error,
  onKeyNameChange,
  onCreateKey,
  onCopySecret,
  onDismissSecret,
  onRevokeKey,
}: ApiKeysViewProps) {
  const duplicateName =
    keyName.trim().length > 0 &&
    keys.some((item) => item.name === keyName.trim());

  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading
          title={
            oneTimeSecret
              ? "Step 2 of 2: Copy your API key"
              : "Step 1 of 2: Create an API key"
          }
        />
        {oneTimeSecret ? (
          <div className="secret-panel" role="status">
            <div>
              <strong>Copy this API key now</strong>
              <p>
                This secret is shown once. Copy it before leaving this step.
              </p>
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
              Create a user-bound key with the scopes needed by this console. The
              secret is shown once.
            </p>
            <form className="key-form" onSubmit={onCreateKey}>
              <label htmlFor="key-name">Key name</label>
              <input
                id="key-name"
                aria-invalid={duplicateName}
                onChange={(event) => onKeyNameChange(event.target.value)}
                placeholder="Dashboard integration"
                required
                value={keyName}
              />
              {duplicateName && (
                <InlineError message="An API key with this name already exists." />
              )}
              <div className="scope-list">
                <span>domains:read</span>
                <span>delivery:read</span>
                <span>delivery:write</span>
              </div>
              <button
                className="button button-primary"
                disabled={duplicateName}
                type="submit"
              >
                Create key
              </button>
            </form>
          </>
        )}
      </section>
      <section className="panel full-width">
        <PanelHeading
          title="Your API keys"
          trailing={`${keys.length} shown · latest 50`}
        />
        {error ? (
          <InlineError message={error} />
        ) : loading ? (
          <EmptyState loading message="Loading API keys..." />
        ) : keys.length === 0 ? (
          <EmptyState message="No API keys are available to this account." />
        ) : (
          <div className="key-list">
            {keys.map((item) => (
              <div className="key-item" key={item.id}>
                <span className="key-symbol" aria-hidden="true">
                  KEY
                </span>
                <span className="key-details">
                  <strong>{item.name}</strong>
                  <small>
                    {item.prefix} · {item.scopes.join(", ")}
                  </small>
                </span>
                <button
                  className="button button-danger-quiet"
                  onClick={() => onRevokeKey(item.id)}
                  type="button"
                >
                  Revoke
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
