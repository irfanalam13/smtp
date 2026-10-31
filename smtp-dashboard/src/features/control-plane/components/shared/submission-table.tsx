import type { Submission } from "../../types/control-plane";
import { formatTime, statusLabel } from "../../utils/formatters";
import { EmptyState, InlineError } from "./feedback-states";

export function SubmissionTable({
  items,
  loading,
  error,
  onRetry,
}: {
  items: Submission[];
  loading: boolean;
  error?: string;
  onRetry?: () => void;
}) {
  if (error) {
    return <InlineError message={error} onRetry={onRetry} />;
  }
  if (loading) {
    return <EmptyState loading message="Loading submissions..." />;
  }
  if (items.length === 0) {
    return <EmptyState message="Submitted messages will appear here." />;
  }
  return (
    <div
      aria-label="Recent submission activity"
      className="table-scroll"
      role="region"
      tabIndex={0}
    >
      <table className="submission-table">
        <thead>
          <tr>
            <th>Message</th>
            <th>Recipient</th>
            <th>Status</th>
            <th>Submitted</th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.id}>
              <td>
                <strong>{item.subject || "(no subject)"}</strong>
                <small>From {item.from}</small>
              </td>
              <td
                title={[
                  item.to.length ? `To: ${item.to.join(", ")}` : "",
                  item.cc?.length ? `Cc: ${item.cc.join(", ")}` : "",
                  item.bcc?.length ? `Bcc: ${item.bcc.join(", ")}` : "",
                ]
                  .filter(Boolean)
                  .join(" · ")}
              >
                {[
                  item.to.length ? `To: ${item.to.join(", ")}` : "",
                  item.cc?.length ? `Cc: ${item.cc.join(", ")}` : "",
                  item.bcc?.length ? `Bcc: ${item.bcc.join(", ")}` : "",
                ]
                  .filter(Boolean)
                  .join(" · ") || "—"}
              </td>
              <td>
                <span className={`status-badge ${item.status}`}>
                  <span className="badge-dot" />
                  {statusLabel(item.status)}
                </span>
              </td>
              <td className="table-date">{formatTime(item.created_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
