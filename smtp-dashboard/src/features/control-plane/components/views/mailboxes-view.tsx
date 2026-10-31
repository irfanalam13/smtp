import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime } from "../../utils/formatters";
import type { Mailbox, Message } from "../../types/control-plane";

type MailboxesViewProps = {
  mailboxes: Mailbox[];
  messages: Message[];
  selectedMailbox: string;
  selectedMessage: Message | null;
  selectedMessageId: string;
  loading: boolean;
  messagesLoading: boolean;
  messageDetailLoading: boolean;
  error?: string;
  messageError?: string;
  onSelectMailbox: (mailboxId: string) => void;
  onSelectMessage: (messageId: string) => void;
  onRetry: () => void;
  onRetryMessage: () => void;
};

export function MailboxesView({
  mailboxes,
  messages,
  selectedMailbox,
  selectedMessage,
  selectedMessageId,
  loading,
  messagesLoading,
  messageDetailLoading,
  error,
  messageError,
  onSelectMailbox,
  onSelectMessage,
  onRetry,
  onRetryMessage,
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
        Only mailboxes returned for this authenticated user are shown. Reads
        require a durable user-bound principal, <code>mailbox:read</code>, and
        mailbox ownership or an explicit grant.
      </p>
      {error ? (
        <InlineError message={error} onRetry={onRetry} />
      ) : loading ? (
        <EmptyState loading message="Loading authorized mailboxes..." />
      ) : mailboxes.length === 0 ? (
        <EmptyState message="No mailboxes are available to this account." />
      ) : messagesLoading ? (
        <EmptyState loading message="Loading message metadata..." />
      ) : messages.length === 0 ? (
        <EmptyState message="This mailbox does not contain any messages." />
      ) : (
        <>
          <div aria-label="Mailbox messages" className="mail-list" role="list">
            {messages.map((item) => (
              <button
                aria-current={
                  selectedMessageId === item.id ? "true" : undefined
                }
                className={`mail-item ${selectedMessageId === item.id ? "selected" : ""}`}
                key={item.id}
                onClick={() => onSelectMessage(item.id)}
                type="button"
              >
                <span className="mail-avatar" aria-hidden="true">
                  {(item.sender[0] ?? "?").toUpperCase()}
                </span>
                <span className="mail-summary">
                  <strong>{item.subject || "(no subject)"}</strong>
                  <small>From {item.sender || "Unknown sender"}</small>
                </span>
                <span className="mail-meta">
                  <time dateTime={item.received_at}>
                    {formatTime(item.received_at)}
                  </time>
                  <small>{item.size.toLocaleString()} bytes</small>
                </span>
              </button>
            ))}
          </div>

          {messageError && (
            <InlineError message={messageError} onRetry={onRetryMessage} />
          )}
          {messageDetailLoading ? (
            <EmptyState loading message="Loading message details..." />
          ) : (
            selectedMessage && (
              <section
                aria-live="polite"
                className="message-detail"
                aria-labelledby="message-detail-heading"
              >
                <h3 id="message-detail-heading">
                  {selectedMessage.subject || "(no subject)"}
                </h3>
                <dl>
                  <div>
                    <dt>From</dt>
                    <dd>{selectedMessage.sender || "Unknown sender"}</dd>
                  </div>
                  <div>
                    <dt>Received</dt>
                    <dd>{formatTime(selectedMessage.received_at)}</dd>
                  </div>
                  <div>
                    <dt>Stored size</dt>
                    <dd>{selectedMessage.size.toLocaleString()} bytes</dd>
                  </div>
                  <div>
                    <dt>Message ID</dt>
                    <dd>
                      <code>{selectedMessage.id}</code>
                    </dd>
                  </div>
                </dl>
                <p>
                  This view displays API-provided message metadata only; it does
                  not render message HTML or download message content.
                </p>
              </section>
            )
          )}
        </>
      )}
    </section>
  );
}
