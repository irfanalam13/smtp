import type { ReactNode } from "react";
import type { Submission } from "../types/control-plane";

export function statusLabel(status: string): string {
  return status.replaceAll("_", " ");
}

export function formatTime(value: string): string {
  return new Date(value).toLocaleString();
}

export function StatCard({
  label,
  value,
  detail,
  tone,
  icon,
}: {
  label: string;
  value: string;
  detail: string;
  tone: string;
  icon: string;
}) {
  return (
    <article className="stat-card">
      <div className={`stat-icon ${tone}`}>{icon}</div>
      <p>{label}</p>
      <div className="stat-value">{value}</div>
      <small>{detail}</small>
    </article>
  );
}

export function PanelHeading({
  title,
  action,
  trailing,
}: {
  title: string;
  action?: ReactNode;
  trailing?: ReactNode;
}) {
  return (
    <div className="panel-heading">
      <h2>{title}</h2>
      {action ??
        (trailing ? <div className="panel-trailing">{trailing}</div> : null)}
    </div>
  );
}

export function InlineError({ message }: { message: string }) {
  return (
    <p className="inline-error" role="status">
      {message}
    </p>
  );
}

export function EmptyState({
  message,
  loading = false,
}: {
  message: string;
  loading?: boolean;
}) {
  return (
    <div className="empty-state" aria-live="polite">
      {loading && <span className="spinner" aria-hidden="true" />}
      <p>{message}</p>
    </div>
  );
}

export function SubmissionTable({
  items,
  loading,
  error,
}: {
  items: Submission[];
  loading: boolean;
  error?: string;
}) {
  if (error) {
    return <InlineError message={error} />;
  }
  if (loading) {
    return <EmptyState loading message="Loading submissions..." />;
  }
  if (items.length === 0) {
    return <EmptyState message="Submitted messages will appear here." />;
  }
  return (
    <div className="table-scroll">
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
              <td>{item.to[0] ?? "—"}</td>
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
