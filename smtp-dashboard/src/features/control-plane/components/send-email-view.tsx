import { PanelHeading, SubmissionTable } from "./dashboard-primitives";
import type { FormEvent } from "react";
import type { LoadErrors, Submission } from "../types/control-plane";

type SendEmailViewProps = {
  sender: string;
  recipients: string;
  subject: string;
  body: string;
  sending: boolean;
  loading: boolean;
  submissions: Submission[];
  loadErrors: LoadErrors;
  onSenderChange: (value: string) => void;
  onRecipientsChange: (value: string) => void;
  onSubjectChange: (value: string) => void;
  onBodyChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
};

export function SendEmailView({
  sender,
  recipients,
  subject,
  body,
  sending,
  loading,
  submissions,
  loadErrors,
  onSenderChange,
  onRecipientsChange,
  onSubjectChange,
  onBodyChange,
  onSubmit,
}: SendEmailViewProps) {
  return (
    <div className="send-grid">
      <section className="panel composer-panel">
        <PanelHeading title="Compose message" />
        <form className="composer-form" onSubmit={onSubmit}>
          <label htmlFor="sender">From</label>
          <input
            autoComplete="email"
            id="sender"
            onChange={(event) => onSenderChange(event.target.value)}
            placeholder="you@your-domain.com"
            required
            type="email"
            value={sender}
          />
          <label htmlFor="recipients">To</label>
          <textarea
            id="recipients"
            onChange={(event) => onRecipientsChange(event.target.value)}
            placeholder="recipient@example.com, another@example.com"
            required
            rows={2}
            value={recipients}
          />
          <label htmlFor="subject">Subject</label>
          <input
            id="subject"
            onChange={(event) => onSubjectChange(event.target.value)}
            placeholder="A short subject line"
            value={subject}
          />
          <label htmlFor="message-body">Message</label>
          <textarea
            className="message-body"
            id="message-body"
            onChange={(event) => onBodyChange(event.target.value)}
            placeholder="Write your email..."
            required
            rows={10}
            value={body}
          />
          <div className="composer-footer">
            <span>Plain text · up to 10 MB including headers</span>
            <button
              className="button button-primary"
              disabled={sending}
              type="submit"
            >
              {sending ? "Submitting..." : "Add to queue"}
              {!sending && <span aria-hidden="true">→</span>}
            </button>
          </div>
        </form>
      </section>
      <section className="panel send-help">
        <PanelHeading title="Before you send" />
        <ul className="checklist">
          <li>
            <span className="checkmark">1</span>
            <span>
              <strong>Use an authorized sender</strong>
              <small>Only send from an address you control.</small>
            </span>
          </li>
          <li>
            <span className="checkmark">2</span>
            <span>
              <strong>Use a scoped API credential</strong>
              <small>The key needs the delivery:write scope.</small>
            </span>
          </li>
          <li>
            <span className="checkmark">3</span>
            <span>
              <strong>Check the queue status</strong>
              <small>Queued does not mean delivered.</small>
            </span>
          </li>
        </ul>
        <div className="subtle-callout">
          External SMTP delivery is disabled in this environment. The local
          worker records simulated outcomes only.
        </div>
      </section>
      <section className="panel full-width">
        <PanelHeading title="Recent queue activity" />
        <SubmissionTable
          items={submissions}
          error={loadErrors.submissions}
          loading={loading}
        />
      </section>
    </div>
  );
}
