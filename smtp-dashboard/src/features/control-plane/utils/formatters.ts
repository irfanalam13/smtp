const statusNames: Record<string, string> = {
  action_required: "Action required",
  pending_verification: "Pending verification",
  retry_scheduled: "Retry scheduled",
  simulated_success: "Simulated (not delivered)",
  simulated_succeeded: "Simulated (not delivered)",
  terminal_failed: "Failed",
};

export function formatTime(value?: string | null): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function statusLabel(status: string): string {
  return (
    statusNames[status] ??
    status
      .replaceAll("_", " ")
      .replace(/\b\w/g, (letter) => letter.toUpperCase())
  );
}
