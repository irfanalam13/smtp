"use client";

import { useState } from "react";
import { ConfirmationDialog } from "@/components/ui/confirmation-dialog";
import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime } from "../../utils/formatters";
import type { Domain, Mailbox } from "../../types/control-plane";
import type { useMailboxManagement } from "../../hooks/use-mailbox-management";

type Management = ReturnType<typeof useMailboxManagement>;
type MailboxManagementViewProps = {
  mailboxes: Mailbox[];
  domains: Domain[];
  management: Management;
  onRetry: () => void;
};

const systemFlags = [
  "\\Seen",
  "\\Answered",
  "\\Flagged",
  "\\Deleted",
  "\\Draft",
];

type PendingConfirmation = {
  title: string;
  description: string;
  confirmLabel: string;
  action: () => void;
};

export function MailboxManagementView({
  mailboxes,
  domains,
  management: state,
  onRetry,
}: MailboxManagementViewProps) {
  const [flagEdits, setFlagEdits] = useState<Record<string, string[]>>({});
  const [pendingConfirmation, setPendingConfirmation] =
    useState<PendingConfirmation | null>(null);
  const selectedMessageId = state.selectedMessage?.id ?? "";
  const editedFlags = state.selectedMessage
    ? (flagEdits[selectedMessageId] ?? state.selectedMessage.flags)
    : [];

  const verifiedDomains = domains.filter(
    (domain) => domain.status === "verified",
  );
  const domainOptions = verifiedDomains.length > 0 ? verifiedDomains : domains;

  return (
    <div className="content-grid">
      <section className="panel">
        <PanelHeading title="Provision a mailbox" />
        <p className="panel-description">
          Requires a durable user-bound account with <code>mailbox:manage</code>
          . Provisioning creates the mailbox and its default folders together.
        </p>
        {domainOptions.length === 0 ? (
          <EmptyState message="Register a domain before provisioning a mailbox." />
        ) : (
          <form className="key-form" onSubmit={state.create}>
            <label htmlFor="new-mailbox-domain">Verified domain</label>
            <select
              id="new-mailbox-domain"
              onChange={(event) => state.setCreateDomainId(event.target.value)}
              value={state.createDomainId}
            >
              {domainOptions.map((domain) => (
                <option key={domain.id} value={domain.id}>
                  {domain.domain} ({domain.status})
                </option>
              ))}
            </select>
            <label htmlFor="new-mailbox-local-part">Address name</label>
            <input
              id="new-mailbox-local-part"
              maxLength={64}
              onChange={(event) => state.setCreateLocalPart(event.target.value)}
              placeholder="support"
              required
              value={state.createLocalPart}
            />
            <label htmlFor="new-mailbox-display-name">Display name</label>
            <input
              id="new-mailbox-display-name"
              maxLength={160}
              onChange={(event) =>
                state.setCreateDisplayName(event.target.value)
              }
              value={state.createDisplayName}
            />
            <label htmlFor="new-mailbox-kind">Mailbox type</label>
            <select
              id="new-mailbox-kind"
              onChange={(event) =>
                state.setCreateKind(event.target.value as "user" | "shared")
              }
              value={state.createKind}
            >
              <option value="user">User</option>
              <option value="shared">Shared</option>
            </select>
            <label htmlFor="new-mailbox-quota">Quota in bytes</label>
            <input
              id="new-mailbox-quota"
              min="1"
              onChange={(event) => state.setCreateQuota(event.target.value)}
              required
              type="number"
              value={state.createQuota}
            />
            {state.createKind === "user" && (
              <>
                <label htmlFor="new-mailbox-owner">
                  Owner user ID (optional)
                </label>
                <input
                  id="new-mailbox-owner"
                  onChange={(event) =>
                    state.setCreateOwnerId(event.target.value)
                  }
                  placeholder="Defaults to the current user"
                  value={state.createOwnerId}
                />
              </>
            )}
            <button
              className="button button-primary"
              disabled={state.busyAction !== ""}
              type="submit"
            >
              {state.busyAction === "mailbox:create"
                ? "Provisioning..."
                : "Provision mailbox"}
            </button>
          </form>
        )}
      </section>

      <section className="panel">
        <PanelHeading
          title="Manage a mailbox"
          trailing={
            <button
              className="button button-quiet"
              disabled={state.loading}
              onClick={onRetry}
              type="button"
            >
              Refresh
            </button>
          }
        />
        <label htmlFor="managed-mailbox">Mailbox</label>
        <select
          id="managed-mailbox"
          onChange={(event) => state.setMailboxId(event.target.value)}
          value={state.mailboxId}
        >
          {mailboxes.length === 0 && <option value="">No mailboxes</option>}
          {mailboxes.map((mailbox) => (
            <option key={mailbox.id} value={mailbox.id}>
              {mailbox.address} · {mailbox.status}
            </option>
          ))}
        </select>
        {state.error && <InlineError message={state.error} onRetry={onRetry} />}
        {state.message && (
          <p className="notice notice-success" role="status">
            {state.message}
          </p>
        )}
        {state.loading ? (
          <EmptyState loading message="Loading mailbox resources..." />
        ) : state.details ? (
          <div className="message-detail">
            <h3>{state.details.display_name || state.details.address}</h3>
            <dl>
              <div>
                <dt>Address</dt>
                <dd>{state.details.address}</dd>
              </div>
              <div>
                <dt>Mailbox ID</dt>
                <dd>
                  <code>{state.details.id}</code>
                </dd>
              </div>
              <div>
                <dt>Type / status</dt>
                <dd>
                  {state.details.kind} / {state.details.status}
                </dd>
              </div>
              <div>
                <dt>Stored bytes</dt>
                <dd>{state.details.used_bytes.toLocaleString()}</dd>
              </div>
              <div>
                <dt>Created</dt>
                <dd>{formatTime(state.details.created_at)}</dd>
              </div>
            </dl>
            <div className="key-actions">
              {state.details.status === "active" ? (
                <button
                  className="button button-secondary"
                  disabled={state.busyAction !== ""}
                  onClick={() => void state.setStatus("suspended")}
                  type="button"
                >
                  Suspend
                </button>
              ) : state.details.status === "suspended" ? (
                <button
                  className="button button-secondary"
                  disabled={state.busyAction !== ""}
                  onClick={() => void state.setStatus("active")}
                  type="button"
                >
                  Reactivate
                </button>
              ) : null}
              {state.details.status !== "deleting" && (
                <button
                  className="button button-danger"
                  disabled={state.busyAction !== ""}
                  onClick={() =>
                    setPendingConfirmation({
                      title: "Mark this mailbox for deletion?",
                      description:
                        "The mailbox will be tombstoned. Its data is not immediately purged.",
                      confirmLabel: "Mark for deletion",
                      action: () => void state.setStatus("deleting"),
                    })
                  }
                  type="button"
                >
                  Mark for deletion
                </button>
              )}
            </div>
          </div>
        ) : mailboxes.length === 0 ? (
          <EmptyState message="No mailbox is available to manage." />
        ) : null}
      </section>

      {state.mailboxId && (
        <>
          <section className="panel">
            <PanelHeading
              title="Quota"
              trailing={
                state.quota
                  ? `${state.quota.used_bytes.toLocaleString()} / ${state.quota.quota_bytes.toLocaleString()} bytes`
                  : undefined
              }
            />
            <p className="panel-description">
              Requires <code>mailbox:manage</code> to change. The quota cannot
              be set below current usage.
            </p>
            <form className="inline-form" onSubmit={state.saveQuota}>
              <label className="visually-hidden" htmlFor="mailbox-quota-bytes">
                New quota in bytes
              </label>
              <input
                id="mailbox-quota-bytes"
                min="1"
                onChange={(event) => state.setQuotaInput(event.target.value)}
                required
                type="number"
                value={state.quotaInput}
              />
              <button
                className="button button-secondary"
                disabled={state.busyAction !== ""}
                type="submit"
              >
                Update quota
              </button>
            </form>
            {state.quota && (
              <progress
                aria-label="Mailbox quota usage"
                max={state.quota.quota_bytes}
                value={state.quota.used_bytes}
              />
            )}
          </section>

          <section className="panel">
            <PanelHeading
              title="Folders"
              trailing={`${state.folders.length} folders`}
            />
            <form className="inline-form" onSubmit={state.addFolder}>
              <label className="visually-hidden" htmlFor="folder-name">
                Folder name
              </label>
              <input
                id="folder-name"
                maxLength={255}
                onChange={(event) => state.setFolderName(event.target.value)}
                placeholder="Folder name"
                required
                value={state.folderName}
              />
              <select
                aria-label="Parent folder"
                onChange={(event) =>
                  state.setFolderParentId(event.target.value)
                }
                value={state.folderParentId}
              >
                <option value="">Root folder</option>
                {state.folders.map((folder) => (
                  <option key={folder.id} value={folder.id}>
                    {folder.name}
                  </option>
                ))}
              </select>
              <select
                aria-label="Special folder role"
                onChange={(event) =>
                  state.setFolderSpecialUse(event.target.value)
                }
                value={state.folderSpecialUse}
              >
                <option value="">No special role</option>
                {["\\Sent", "\\Drafts", "\\Trash", "\\Archive", "\\Junk"].map(
                  (role) => (
                    <option key={role} value={role}>
                      {role}
                    </option>
                  ),
                )}
              </select>
              <button
                className="button button-secondary"
                disabled={state.busyAction !== ""}
                type="submit"
              >
                Add folder
              </button>
            </form>
            <div className="key-list">
              {state.folders.map((folder) => (
                <article className="key-item" key={folder.id}>
                  <span className="key-details">
                    <strong>{folder.name}</strong>
                    <small>
                      {folder.special_use ?? "Folder"} · UIDVALIDITY{" "}
                      {folder.uid_validity} · next UID {folder.uid_next}
                    </small>
                  </span>
                  <div className="key-actions">
                    <button
                      className="button button-secondary"
                      disabled={
                        state.busyAction !== "" ||
                        Boolean(folder.special_use) ||
                        folder.name === "INBOX"
                      }
                      onClick={() => void state.renameFolder(folder)}
                      type="button"
                    >
                      Rename
                    </button>
                    <button
                      className="button button-secondary"
                      disabled={state.busyAction !== ""}
                      onClick={() => void state.expungeFolder(folder.id)}
                      type="button"
                    >
                      Expunge deleted
                    </button>
                    <button
                      className="button button-danger"
                      disabled={
                        state.busyAction !== "" ||
                        Boolean(folder.special_use) ||
                        folder.name === "INBOX"
                      }
                      onClick={() =>
                        setPendingConfirmation({
                          title: `Delete ${folder.name}?`,
                          description:
                            "Only an empty, non-system folder can be deleted.",
                          confirmLabel: "Delete folder",
                          action: () => void state.removeFolder(folder),
                        })
                      }
                      type="button"
                    >
                      Delete
                    </button>
                  </div>
                </article>
              ))}
            </div>
          </section>

          <section className="panel">
            <PanelHeading
              title="Aliases"
              trailing={`${state.aliases.length} aliases`}
            />
            <form className="inline-form" onSubmit={state.addAlias}>
              <label className="visually-hidden" htmlFor="alias-local-part">
                Alias local part
              </label>
              <input
                id="alias-local-part"
                maxLength={64}
                onChange={(event) =>
                  state.setAliasLocalPart(event.target.value)
                }
                placeholder="Alias name"
                required
                value={state.aliasLocalPart}
              />
              <select
                aria-label="Alias domain"
                onChange={(event) => state.setAliasDomainId(event.target.value)}
                value={state.aliasDomainId}
              >
                {domainOptions.map((domain) => (
                  <option key={domain.id} value={domain.id}>
                    {domain.domain}
                  </option>
                ))}
              </select>
              <button
                className="button button-secondary"
                disabled={state.busyAction !== "" || domainOptions.length === 0}
                type="submit"
              >
                Add alias
              </button>
            </form>
            {state.aliases.map((alias) => (
              <article className="key-item" key={alias.id}>
                <span className="key-details">
                  <strong>{alias.address}</strong>
                  <small>{alias.status}</small>
                </span>
                <button
                  className="button button-danger"
                  disabled={state.busyAction !== ""}
                  onClick={() =>
                    setPendingConfirmation({
                      title: `Remove ${alias.address}?`,
                      description:
                        "Mail sent to this alias will no longer be accepted for this mailbox.",
                      confirmLabel: "Remove alias",
                      action: () => void state.removeAlias(alias),
                    })
                  }
                  type="button"
                >
                  Remove
                </button>
              </article>
            ))}
          </section>

          <section className="panel">
            <PanelHeading
              title="Mailbox access"
              trailing={`${state.permissions.length} grants`}
            />
            <form className="key-form" onSubmit={state.grantPermission}>
              <label htmlFor="permission-user-id">User ID</label>
              <input
                id="permission-user-id"
                onChange={(event) =>
                  state.setPermissionUserId(event.target.value)
                }
                placeholder="UUID"
                required
                value={state.permissionUserId}
              />
              <label htmlFor="permission-level">Permission</label>
              <select
                id="permission-level"
                onChange={(event) =>
                  state.setPermissionLevel(
                    event.target.value as "read" | "read_write" | "manage",
                  )
                }
                value={state.permissionLevel}
              >
                <option value="read">Read</option>
                <option value="read_write">Read and write</option>
                <option value="manage">Manage</option>
              </select>
              <button
                className="button button-secondary"
                disabled={state.busyAction !== ""}
                type="submit"
              >
                Grant access
              </button>
            </form>
            <div className="key-list">
              {state.permissions.map((permission) => (
                <article className="key-item" key={permission.user_id}>
                  <span className="key-details">
                    <strong>{permission.user_id}</strong>
                    <small>
                      {permission.permission} · granted{" "}
                      {formatTime(permission.created_at)}
                    </small>
                  </span>
                  <button
                    className="button button-danger"
                    disabled={state.busyAction !== ""}
                    onClick={() =>
                      setPendingConfirmation({
                        title: "Revoke mailbox access?",
                        description: `This removes ${permission.user_id}'s ${permission.permission} access immediately.`,
                        confirmLabel: "Revoke access",
                        action: () => void state.removePermission(permission),
                      })
                    }
                    type="button"
                  >
                    Revoke
                  </button>
                </article>
              ))}
            </div>
          </section>

          <section className="panel">
            <PanelHeading
              title="Labels"
              trailing={`${state.labels.length} labels`}
            />
            <form className="inline-form" onSubmit={state.addLabel}>
              <label className="visually-hidden" htmlFor="label-name">
                Label name
              </label>
              <input
                id="label-name"
                maxLength={64}
                onChange={(event) => state.setLabelName(event.target.value)}
                placeholder="Label name"
                required
                value={state.labelName}
              />
              <label className="visually-hidden" htmlFor="label-color">
                Label color
              </label>
              <input
                id="label-color"
                onChange={(event) => state.setLabelColor(event.target.value)}
                type="color"
                value={state.labelColor}
              />
              <button
                className="button button-secondary"
                disabled={state.busyAction !== ""}
                type="submit"
              >
                Create label
              </button>
            </form>
            <div className="key-list">
              {state.labels.map((label) => (
                <article className="key-item" key={label.id}>
                  <span className="key-details">
                    <strong>{label.name}</strong>
                    <small>
                      {label.kind} · {label.color ?? "no color"}
                    </small>
                  </span>
                </article>
              ))}
            </div>
          </section>

          <section className="panel full-width">
            <PanelHeading
              title="Mailbox messages"
              trailing={`${state.messages.length} shown`}
            />
            <p className="panel-description">
              Message content is only fetched when you request it. Access to
              content, headers, and body is audited by the service.
            </p>
            <div className="mail-list" role="list">
              {state.messages.map((item) => (
                <article className="mail-item" key={item.id}>
                  <button
                    className="mail-summary"
                    onClick={() => void state.loadMessage(item.id)}
                    type="button"
                  >
                    <strong>{item.subject || "(no subject)"}</strong>
                    <small>
                      From {item.sender || "Unknown sender"} ·{" "}
                      {item.size.toLocaleString()} bytes
                    </small>
                    <small>{formatTime(item.received_at)}</small>
                  </button>
                  <div className="key-actions">
                    <button
                      className="button button-danger"
                      disabled={state.busyAction !== ""}
                      onClick={() =>
                        setPendingConfirmation({
                          title: "Mark this message as deleted?",
                          description:
                            "The message remains recoverable until its folder is expunged.",
                          confirmLabel: "Mark deleted",
                          action: () => void state.deleteMessage(item),
                        })
                      }
                      type="button"
                    >
                      Mark deleted
                    </button>
                  </div>
                </article>
              ))}
            </div>
            {state.messages.length === 0 && (
              <EmptyState message="This mailbox has no messages." />
            )}

            {state.selectedMessage && (
              <section className="message-detail">
                <PanelHeading
                  title={state.selectedMessage.subject || "Selected message"}
                  trailing={`UID ${state.selectedMessage.uid}`}
                />
                <p>
                  <code>{state.selectedMessage.id}</code> ·{" "}
                  {state.selectedMessage.sender || "Unknown sender"} · flags:{" "}
                  {state.selectedMessage.flags.join(" ") || "none"}
                </p>
                <div className="key-actions">
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== ""}
                    onClick={() => void state.readMessagePart("headers")}
                    type="button"
                  >
                    Read headers
                  </button>
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== ""}
                    onClick={() => void state.readMessagePart("body")}
                    type="button"
                  >
                    Read body
                  </button>
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== ""}
                    onClick={() => void state.downloadMessage()}
                    type="button"
                  >
                    Download .eml
                  </button>
                </div>
                {state.messagePart && (
                  <pre className="message-source">{state.messagePart}</pre>
                )}
                <fieldset className="scope-options">
                  <legend>System flags</legend>
                  {systemFlags.map((flag) => (
                    <label className="scope-option" key={flag}>
                      <input
                        checked={editedFlags.includes(flag)}
                        onChange={(event) =>
                          setFlagEdits((current) => ({
                            ...current,
                            [selectedMessageId]: event.target.checked
                              ? [...editedFlags, flag]
                              : editedFlags.filter((item) => item !== flag),
                          }))
                        }
                        type="checkbox"
                      />
                      <code>{flag}</code>
                    </label>
                  ))}
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== ""}
                    onClick={() => void state.saveFlags(editedFlags)}
                    type="button"
                  >
                    Save flags
                  </button>
                </fieldset>
                <div className="key-actions">
                  <label htmlFor="target-folder">Target folder</label>
                  <select
                    id="target-folder"
                    onChange={(event) =>
                      state.setTargetFolderId(event.target.value)
                    }
                    value={state.targetFolderId}
                  >
                    <option value="">Choose folder</option>
                    {state.folders.map((folder) => (
                      <option key={folder.id} value={folder.id}>
                        {folder.name}
                      </option>
                    ))}
                  </select>
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== "" || !state.targetFolderId}
                    onClick={() => void state.transferMessage("copy")}
                    type="button"
                  >
                    Copy
                  </button>
                  <button
                    className="button button-secondary"
                    disabled={state.busyAction !== "" || !state.targetFolderId}
                    onClick={() => void state.transferMessage("move")}
                    type="button"
                  >
                    Move
                  </button>
                </div>
                <div className="key-list">
                  {state.labels.map((label) => (
                    <article className="key-item" key={label.id}>
                      <span className="key-details">
                        <strong>{label.name}</strong>
                      </span>
                      <span className="key-actions">
                        <button
                          className="button button-secondary"
                          disabled={state.busyAction !== ""}
                          onClick={() =>
                            void state.toggleLabel(
                              state.selectedMessage!.id,
                              label.id,
                              false,
                            )
                          }
                          type="button"
                        >
                          Assign
                        </button>
                        <button
                          className="button button-quiet"
                          disabled={state.busyAction !== ""}
                          onClick={() =>
                            void state.toggleLabel(
                              state.selectedMessage!.id,
                              label.id,
                              true,
                            )
                          }
                          type="button"
                        >
                          Remove
                        </button>
                      </span>
                    </article>
                  ))}
                </div>
              </section>
            )}

            <form className="key-form" onSubmit={state.append}>
              <label htmlFor="raw-mailbox-message">
                Append RFC 5322 message
              </label>
              <textarea
                id="raw-mailbox-message"
                className="message-body"
                onChange={(event) => state.setRawMessage(event.target.value)}
                placeholder={
                  "From: sender@example.com\nTo: recipient@example.com\nSubject: Example\n\nMessage body"
                }
                required
                rows={8}
                value={state.rawMessage}
              />
              <button
                className="button button-primary"
                disabled={state.busyAction !== ""}
                type="submit"
              >
                Append message
              </button>
            </form>
          </section>
        </>
      )}
      <ConfirmationDialog
        confirmLabel={pendingConfirmation?.confirmLabel ?? "Confirm"}
        description={pendingConfirmation?.description ?? ""}
        onCancel={() => setPendingConfirmation(null)}
        onConfirm={() => {
          pendingConfirmation?.action();
          setPendingConfirmation(null);
        }}
        open={Boolean(pendingConfirmation)}
        title={pendingConfirmation?.title ?? "Confirm action"}
      />
    </div>
  );
}
