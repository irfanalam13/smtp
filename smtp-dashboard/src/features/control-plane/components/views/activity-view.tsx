import { EmptyState, InlineError } from "../shared/feedback-states";
import { PanelHeading } from "../shared/panel-heading";
import { formatTime, statusLabel } from "../../utils/formatters";
import type { Submission, SubmissionAttempt } from "../../types/control-plane";

type ActivityViewProps = {
  submissions: Submission[];
  selectedSubmissionId: string;
  attempts: SubmissionAttempt[];
  loading: boolean;
  attemptsLoading: boolean;
  submissionsError?: string;
  attemptsError?: string;
  onRetrySubmissions: () => void;
  onRetryAttempts: () => void;
  onSelectSubmission: (submissionId: string) => void;
};

export function ActivityView({
  submissions,
  selectedSubmissionId,
  attempts,
  loading,
  attemptsLoading,
  submissionsError,
  attemptsError,
  onRetrySubmissions,
  onRetryAttempts,
  onSelectSubmission,
}: ActivityViewProps) {
  const selectedSubmission = submissions.find(
    (submission) => submission.id === selectedSubmissionId,
  );

  return (
    <div className="activity-layout">
      <section className="panel full-width">
        <PanelHeading title="Submission activity" trailing="SMTP delivery" />
        <p className="panel-description">
          Review SMTP provider acceptance and retry history for each recipient.
        </p>
        {submissionsError ? (
          <InlineError
            message={submissionsError}
            onRetry={onRetrySubmissions}
          />
        ) : loading ? (
          <EmptyState loading message="Loading submissions..." />
        ) : submissions.length === 0 ? (
          <EmptyState message="No submissions are available to inspect." />
        ) : (
          <>
            <label htmlFor="activity-submission">Submission</label>
            <select
              id="activity-submission"
              onChange={(event) => onSelectSubmission(event.target.value)}
              value={selectedSubmissionId}
            >
              <option value="">Select a submission</option>
              {submissions.map((submission) => (
                <option key={submission.id} value={submission.id}>
                  {submission.subject || "(no subject)"} ·{" "}
                  {submission.id.slice(0, 8)} · {statusLabel(submission.status)}
                </option>
              ))}
            </select>
          </>
        )}
      </section>

      {selectedSubmission && (
        <section className="panel full-width">
          <PanelHeading
            title={selectedSubmission.subject || "(no subject)"}
            trailing={statusLabel(selectedSubmission.status)}
          />
          <p className="activity-meta">
            {selectedSubmission.from}
            {selectedSubmission.to.length > 0 &&
              ` · To: ${selectedSubmission.to.join(", ")}`}
            {selectedSubmission.cc.length > 0 &&
              ` · Cc: ${selectedSubmission.cc.join(", ")}`}
            {selectedSubmission.bcc.length > 0 &&
              ` · Bcc: ${selectedSubmission.bcc.join(", ")}`}
          </p>
          <p className="activity-meta">
            Submitted {formatTime(selectedSubmission.created_at)} · ID{" "}
            <code>{selectedSubmission.id}</code>
          </p>
          {attemptsLoading ? (
            <EmptyState loading message="Loading delivery attempts..." />
          ) : attemptsError ? (
            <InlineError message={attemptsError} onRetry={onRetryAttempts} />
          ) : attempts.length === 0 ? (
            <EmptyState message="No delivery attempts have been recorded yet." />
          ) : (
            <ol className="attempt-list">
              {attempts.map((attempt) => (
                <li className="attempt-item" key={attempt.id}>
                  <div className="attempt-heading">
                    <strong>
                      Attempt {attempt.attempt_number} · {attempt.recipient}
                    </strong>
                    <span className={`status-badge ${attempt.outcome}`}>
                      <span className="badge-dot" />
                      {statusLabel(attempt.outcome)}
                    </span>
                  </div>
                  <p>
                    Started {formatTime(attempt.started_at)}
                    {attempt.finished_at &&
                      ` · Finished ${formatTime(attempt.finished_at)}`}
                  </p>
                  {attempt.error_category && (
                    <p>Error category: {statusLabel(attempt.error_category)}</p>
                  )}
                  {attempt.smtp_code && <p>SMTP response: {attempt.smtp_code}</p>}
                  {attempt.remote_host && <p>Server: {attempt.remote_host}</p>}
                  {attempt.diagnostic && (
                    <p className="attempt-diagnostic">{attempt.diagnostic}</p>
                  )}
                </li>
              ))}
            </ol>
          )}
        </section>
      )}
    </div>
  );
}
