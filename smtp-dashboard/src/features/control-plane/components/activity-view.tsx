import {
  EmptyState,
  InlineError,
  PanelHeading,
  formatTime,
  statusLabel,
} from "./dashboard-primitives";
import type { Submission, SubmissionAttempt } from "../types/control-plane";

type ActivityViewProps = {
  submissions: Submission[];
  selectedSubmissionId: string;
  attempts: SubmissionAttempt[];
  loading: boolean;
  attemptsLoading: boolean;
  error?: string;
  onSelectSubmission: (submissionId: string) => void;
};

export function ActivityView({
  submissions,
  selectedSubmissionId,
  attempts,
  loading,
  attemptsLoading,
  error,
  onSelectSubmission,
}: ActivityViewProps) {
  const selectedSubmission = submissions.find(
    (submission) => submission.id === selectedSubmissionId,
  );

  return (
    <div className="activity-layout">
      <section className="panel full-width">
        <PanelHeading title="Submission activity" trailing="Queue attempts" />
        <p className="panel-description">
          Review persisted processing attempts for a recent submission.
          Development attempts may be simulated and do not confirm external
          delivery.
        </p>
        {error && <InlineError message={error} />}
        {loading ? (
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
            {selectedSubmission.from} to {selectedSubmission.to.join(", ")}
          </p>
          <p className="activity-meta">
            Submitted {formatTime(selectedSubmission.created_at)} · ID{" "}
            <code>{selectedSubmission.id}</code>
          </p>
          {attemptsLoading ? (
            <EmptyState loading message="Loading queue attempts..." />
          ) : attempts.length === 0 ? (
            <EmptyState message="No processing attempts have been recorded yet." />
          ) : (
            <ol className="attempt-list">
              {attempts.map((attempt) => (
                <li className="attempt-item" key={attempt.id}>
                  <div className="attempt-heading">
                    <strong>Attempt {attempt.attempt_number}</strong>
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
