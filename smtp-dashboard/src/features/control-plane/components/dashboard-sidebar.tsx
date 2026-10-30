"use client";

import { useState } from "react";
import type { DashboardView, Principal } from "../types/control-plane";

type DashboardSidebarProps = {
  brandName: string;
  view: DashboardView;
  onViewChange: (view: DashboardView) => void;
  connected: boolean;
  loading: boolean;
  connectionError: string;
  principal: Principal | null;
};

type NavigationLink = {
  label: string;
  short: string;
  view?: DashboardView;
  children?: NavigationLink[];
  disabled?: boolean;
};

const navigation: NavigationLink[] = [
  { label: "Home", short: "HM", view: "home" },
  {
    label: "Transaction",
    short: "TX",
    children: [
      {
        label: "Email",
        short: "EM",
        children: [
          { label: "Send email", short: "SE", view: "send" },
          { label: "Activity logs", short: "AC", view: "activity" },
          { label: "Statistics", short: "ST", disabled: true },
          { label: "Templates", short: "TP", disabled: true },
        ],
      },
      {
        label: "SMS",
        short: "SMS",
        disabled: true,
        children: [
          { label: "Messages", short: "MS", disabled: true },
          { label: "Statistics", short: "ST", disabled: true },
        ],
      },
    ],
  },
  {
    label: "Deliverability",
    short: "DL",
    children: [
      { label: "Sending domains", short: "DM", view: "domains" },
      { label: "Suppressions", short: "SP", disabled: true },
      { label: "Event webhooks", short: "WH", disabled: true },
    ],
  },
  {
    label: "Inbound",
    short: "IN",
    children: [
      { label: "Mailboxes", short: "MB", view: "mailboxes" },
      { label: "Routing rules", short: "RR", disabled: true },
    ],
  },
  {
    label: "Developers",
    short: "DV",
    children: [
      { label: "API keys", short: "AK", view: "api-keys" },
      { label: "API reference", short: "API", disabled: true },
      { label: "Webhooks", short: "WH", disabled: true },
    ],
  },
  {
    label: "Account",
    short: "AC",
    children: [
      { label: "Credentials", short: "CR", view: "credentials" },
      { label: "Team & access", short: "TM", disabled: true },
      { label: "Usage & billing", short: "UB", disabled: true },
    ],
  },
];

function containsView(item: NavigationLink, view: DashboardView): boolean {
  return (
    item.view === view ||
    (item.children?.some((child) => containsView(child, view)) ?? false)
  );
}

function groupsForView(
  items: NavigationLink[],
  view: DashboardView,
  parents: string[] = [],
): string[] | undefined {
  for (const item of items) {
    if (item.view === view) {
      return parents;
    }
    if (item.children) {
      const groups = groupsForView(item.children, view, [
        ...parents,
        item.label,
      ]);
      if (groups) {
        return groups;
      }
    }
  }
  return undefined;
}

export function DashboardSidebar({
  brandName,
  view,
  onViewChange,
  connected,
  loading,
  connectionError,
  principal,
}: DashboardSidebarProps) {
  const [expanded, setExpanded] = useState<string[]>(
    () => groupsForView(navigation, view) ?? [],
  );

  function toggle(label: string) {
    setExpanded((current) =>
      current.includes(label)
        ? current.filter((item) => item !== label)
        : [...current, label],
    );
  }

  function renderItems(items: NavigationLink[], depth = 0) {
    return items.map((item) => {
      const hasChildren = Boolean(item.children?.length);
      const isExpanded = expanded.includes(item.label);
      const isActive = item.view === view;
      const isGroupActive = containsView(item, view);
      const className = [
        "nav-item",
        depth > 0 ? "nav-item-nested" : "",
        isActive ? "active" : "",
        isGroupActive && hasChildren ? "group-active" : "",
        item.disabled ? "nav-item-disabled" : "",
      ]
        .filter(Boolean)
        .join(" ");

      return (
        <div className="nav-entry" key={item.label}>
          <button
            aria-current={isActive ? "page" : undefined}
            aria-expanded={hasChildren ? isExpanded : undefined}
            className={className}
            disabled={item.disabled && !hasChildren}
            onClick={() => {
              if (hasChildren) {
                toggle(item.label);
              } else if (item.view) {
                onViewChange(item.view);
              }
            }}
            title={item.disabled ? `${item.label} — coming later` : undefined}
            type="button"
          >
            <span className="nav-icon" aria-hidden="true">
              {item.short}
            </span>
            <span className="nav-label">{item.label}</span>
            {item.disabled && <span className="nav-coming-soon">Soon</span>}
            {hasChildren && (
              <span
                className={`nav-chevron ${isExpanded ? "expanded" : ""}`}
                aria-hidden="true"
              />
            )}
          </button>
          {hasChildren && isExpanded && (
            <div className="nav-children">
              {renderItems(item.children ?? [], depth + 1)}
            </div>
          )}
        </div>
      );
    });
  }

  return (
    <aside className="sidebar">
      <a
        className="brand"
        href="#home"
        onClick={(event) => {
          event.preventDefault();
          onViewChange("home");
        }}
      >
        <span className="brand-mark">S</span>
        <span>
          <strong>{brandName}</strong>
          <small>Email platform</small>
        </span>
      </a>

      <nav className="navigation" aria-label="Main navigation">
        {renderItems(navigation)}
      </nav>

      <div className="sidebar-bottom">
        <div className={`connection ${connected ? "online" : ""}`}>
          <span className="connection-dot" />
          <span>
            <strong>
              {connected
                ? "API reachable"
                : loading
                  ? "Connecting"
                  : "API unavailable"}
            </strong>
            <small>
              {connected
                ? "Backend connection"
                : connectionError || "Check API service"}
            </small>
          </span>
        </div>
        <div className="sidebar-user">
          <span className="avatar">
            {(principal?.subject?.[0] ?? "U").toUpperCase()}
          </span>
          <span>
            <strong>{principal?.role ?? "Not signed in"}</strong>
            <small>{principal?.tenant?.name ?? "SMTP workspace"}</small>
          </span>
        </div>
      </div>
    </aside>
  );
}
