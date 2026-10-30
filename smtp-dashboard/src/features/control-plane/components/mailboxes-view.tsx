import {
  EmptyState,
  InlineError,
  PanelHeading,
  formatTime,
} from "./dashboard-primitives";
import type { Mailbox, Message } from "../types/control-plane";

type MailboxesViewProps = {
  mailboxes: Mailbox[];
  messages: Message[];
  selectedMailbox: string;
  loading: boolean;
  error?: string;
  onSelectMailbox: (mailboxId: string) => void;
};

export function MailboxesView({
  mailboxes,
  messages,
  selectedMailbox,
  loading,
  error,
  onSelectMailbox,
}: MailboxesViewProps) {
  return (
    <section className="panel">
      <PanelHeading
        title="Mailbox messages"
        trailing={
          mailboxes.length > 0 ? (
            <label className="mailbox-picker">
              <span className="visually-hidden">Select mailbox</span>
              <select
                onChange={(event) => onSelectMailbox(event.target.value)}
                value={selectedMailbox}
              >
                {mailboxes.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.address}
                  </option>
                ))}
              </select>
            </label>
          ) : undefined
        }
      />
      <p className="panel-description">
        Mailbox data requires a user-bound API key or configured OIDC access
        token, and the signed-in user must own the mailbox or have explicit
        mailbox permission. The local development identity cannot access
        PostgreSQL-backed mailboxes.
      </p>
      {error ? (
        <InlineError message={error} />
      ) : loading ? (
        <EmptyState loading message="Loading mailbox..." />
      ) : mailboxes.length === 0 ? (
        <EmptyState message="No mailboxes are available to this account." />
      ) : messages.length === 0 ? (
        <EmptyState message="This mailbox does not contain any messages." />
      ) : (
        <div className="mail-list">
          {messages.map((item) => (
            <article className="mail-item" key={item.id}>
              <span className="mail-avatar">
                {(item.sender[0] ?? "?").toUpperCase()}
              </span>
              <span className="mail-summary">
                <strong>{item.subject || "(no subject)"}</strong>
                <small>From {item.sender}</small>
              </span>
              <span className="mail-meta">
                <time dateTime={item.received_at}>
                  {formatTime(item.received_at)}
                </time>
                <small>{item.size.toLocaleString()} bytes</small>
              </span>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
