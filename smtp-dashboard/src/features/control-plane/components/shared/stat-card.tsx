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
  tone: "blue" | "green" | "violet" | "amber";
  icon: ReactNode;
}) {
  return (
    <article className="stat-card">
      <div aria-hidden="true" className={`stat-icon ${tone}`}>
        {icon}
      </div>
      <p>{label}</p>
      <output aria-label={label} className="stat-value">
        {value}
      </output>
      <small>{detail}</small>
    </article>
  );
}
import type { ReactNode } from "react";
