import { useState } from "react";
import type { FormEvent } from "react";
import { PanelHeading } from "../shared/panel-heading";
import { SubmissionTable } from "../shared/submission-table";
import type { ComposeField } from "../../utils/validators";
import type {
  LoadErrors,
  SendingConfig,
  Submission,
} from "../../types/control-plane";

type SendEmailViewProps = {
  sender: string;
  recipients: string;
  cc: string;
  bcc: string;
  sendingConfig: SendingConfig;
  subject: string;
  body: string;
  composeErrors: Partial<Record<ComposeField, string>>;
  sending: boolean;
  loading: boolean;
  submissions: Submission[];
  loadErrors: LoadErrors;
  onSenderChange: (value: string) => void;
  onRecipientsChange: (value: string) => void;
  onCcChange: (value: string) => void;
  onBccChange: (value: string) => void;
  onSubjectChange: (value: string) => void;
  onBodyChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onRetry: () => void;
};

function fieldError(id: string, message?: string) {
  return message ? (
    <p className="field-error" id={`${id}-error`} role="alert">
      {message}
    </p>
  ) : null;
}

export function SendEmailView({
  sender,
  recipients,
  cc,
  bcc,
  sendingConfig,
  subject,
  body,
  composeErrors,
  sending,
  loading,
  submissions,
  loadErrors,
  onSenderChange,
  onRecipientsChange,
  onCcChange,
  onBccChange,
  onSubjectChange,
  onBodyChange,
  onSubmit,
  onRetry,
}: SendEmailViewProps) {
  const [showCc, setShowCc] = useState(Boolean(cc));
  const [showBcc, setShowBcc] = useState(Boolean(bcc));
  return (
    <div className="send-grid">
      <section className="panel composer-panel">
        <PanelHeading title="New message" />
        {!sendingConfig.relay_ready && (
          <div className="subtle-callout" role="status">
            {sendingConfig.error
              ? `Could not check the SMTP sending configuration: ${sendingConfig.error}`
              : "Email sending is not configured. An app admin must set up the shared Gmail or Brevo SMTP connection on the server before you can send."}
          </div>
        )}
        <form className="composer-form" onSubmit={onSubmit} noValidate>
          <label htmlFor="sender">From</label>
          <input
            aria-describedby={composeErrors.from ? "sender-error" : undefined}
            aria-invalid={Boolean(composeErrors.from)}
            autoComplete="email"
            id="sender"
            onChange={(event) => onSenderChange(event.target.value)}
            placeholder="Configured sender address"
            readOnly
            required
            type="email"
            value={sender}
          />
          {fieldError("sender", composeErrors.from)}

          <div className="composer-recipient-heading">
            <label htmlFor="recipients">To</label>
            <span>
              <button
                aria-expanded={showCc}
                type="button"
                onClick={() => setShowCc((value) => !value)}
              >
                Cc
              </button>
              <button
                aria-expanded={showBcc}
                type="button"
                onClick={() => setShowBcc((value) => !value)}
              >
                Bcc
              </button>
            </span>
          </div>
          <textarea
            aria-describedby={
              composeErrors.to ? "recipients-error" : undefined
            }
            aria-invalid={Boolean(composeErrors.to)}
            id="recipients"
            onChange={(event) => onRecipientsChange(event.target.value)}
            placeholder="Recipients"
            rows={2}
            value={recipients}
          />
          {fieldError("recipients", composeErrors.to)}

          {showCc && (
            <>
              <label htmlFor="cc">Cc</label>
              <textarea
                aria-describedby={composeErrors.cc ? "cc-error" : undefined}
                aria-invalid={Boolean(composeErrors.cc)}
                id="cc"
                onChange={(event) => onCcChange(event.target.value)}
                placeholder="Cc recipients"
                rows={2}
                value={cc}
              />
              {fieldError("cc", composeErrors.cc)}
            </>
          )}

          {showBcc && (
            <>
              <label htmlFor="bcc">Bcc</label>
              <textarea
                aria-describedby={composeErrors.bcc ? "bcc-error" : undefined}
                aria-invalid={Boolean(composeErrors.bcc)}
                id="bcc"
                onChange={(event) => onBccChange(event.target.value)}
                placeholder="Bcc recipients (hidden from other recipients)"
                rows={2}
                value={bcc}
              />
              {fieldError("bcc", composeErrors.bcc)}
            </>
          )}
          <small className="field-hint">
            Separate up to 100 addresses across To, Cc, and Bcc with commas,
            semicolons, or new lines. Bcc addresses are not shown in the email.
          </small>

          <label htmlFor="subject">Subject</label>
          <input
            aria-describedby={
              composeErrors.subject ? "subject-error" : undefined
            }
            aria-invalid={Boolean(composeErrors.subject)}
            id="subject"
            maxLength={900}
            onChange={(event) => onSubjectChange(event.target.value)}
            placeholder="A short subject line"
            value={subject}
          />
          {fieldError("subject", composeErrors.subject)}

          <label htmlFor="message-body">Message</label>
          <textarea
            aria-describedby={
              composeErrors.body ? "message-body-error" : undefined
            }
            aria-invalid={Boolean(composeErrors.body)}
            className="message-body"
            id="message-body"
            onChange={(event) => onBodyChange(event.target.value)}
            placeholder="Write your plain-text email..."
            required
            rows={10}
            value={body}
          />
          {fieldError("message-body", composeErrors.body)}

          <div className="composer-footer">
            <span>
              {sendingConfig.relay_ready
                ? `Sending as ${sendingConfig.from} · Plain text`
                : "SMTP relay required · Plain text"}
            </span>
            <button
              className="button button-primary"
              disabled={sending || !sendingConfig.relay_ready}
              type="submit"
            >
              {sending ? "Sending..." : "Send email"}
              {!sending && <span aria-hidden="true">→</span>}
            </button>
          </div>
        </form>
      </section>

      <section className="panel send-help">
        <PanelHeading title="Before you send" />
        <ul className="checklist">
          <li>
            <span className="checkmark" aria-hidden="true">
            1
            </span>
            <span>
              <strong>Messages go through the shared SMTP account</strong>
              <small>
                The app admin configures Gmail SMTP or Brevo SMTP on the server.
              </small>
            </span>
          </li>
          <li>
            <span className="checkmark" aria-hidden="true">
            2
            </span>
            <span>
              <strong>Check the delivery status</strong>
              <small>Queue acceptance does not confirm remote delivery.</small>
            </span>
          </li>
        </ul>
        <div className="subtle-callout">
          Bcc addresses are kept out of message headers. The delivery activity
          shows whether the SMTP provider accepted each recipient; provider
          acceptance does not guarantee inbox placement.
        </div>
      </section>

      <section className="panel full-width">
        <PanelHeading title="Recent queue activity" />
        <SubmissionTable
          items={submissions}
          error={loadErrors.submissions}
          loading={loading}
          onRetry={onRetry}
        />
      </section>
    </div>
  );
}
