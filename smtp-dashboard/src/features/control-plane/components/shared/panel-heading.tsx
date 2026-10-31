import type { ReactNode } from "react";

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
