"use client";

import { useLayoutEffect, useRef, useState } from "react";
import {
  ActivityIcon,
  GlobeIcon,
  HomeIcon,
  InboxIcon,
  KeyIcon,
  MailIcon,
  SendIcon,
  ServerIcon,
  SettingsIcon,
  ShieldIcon,
  UserIcon,
  UsersIcon,
  XIcon,
} from "@/components/ui/icons";
import type { KeyboardEvent, ReactNode, RefObject } from "react";
import type {
  DashboardView,
  Principal,
  Tenant,
} from "../../types/control-plane";

type DashboardSidebarProps = {
  brandName: string;
  view: DashboardView;
  onViewChange: (view: DashboardView) => void;
  mobile: boolean;
  mobileOpen: boolean;
  onMobileClose: () => void;
  closeButtonRef: RefObject<HTMLButtonElement | null>;
  connected: boolean;
  loading: boolean;
  connectionError: string;
  principal: Principal | null;
  tenant: Tenant | null;
};

type NavigationLink = {
  label: string;
  icon: ReactNode;
  view?: DashboardView;
  children?: NavigationLink[];
  disabled?: boolean;
  isSoon?: boolean;
};

const navigation: NavigationLink[] = [
  { label: "Home", icon: <HomeIcon />, view: "home" },
  {
    label: "Transaction",
    icon: <SendIcon />,
    children: [
      {
        label: "Email",
        icon: <MailIcon />,
        children: [
          { label: "Send email", icon: <SendIcon />, view: "send" },
          { label: "Activity logs", icon: <ActivityIcon />, view: "activity" },
          {
            label: "Statistics",
            icon: <ActivityIcon />,
            disabled: true,
            isSoon: true,
          },
          {
            label: "Templates",
            icon: <MailIcon />,
            disabled: true,
            isSoon: true,
          },
        ],
      },
      {
        label: "SMS",
        icon: <SendIcon />,
        disabled: true,
        isSoon: true,
      },
    ],
  },
  {
    label: "Deliverability",
    icon: <GlobeIcon />,
    children: [
      { label: "Sending domains", icon: <GlobeIcon />, view: "domains" },
      {
        label: "Suppressions",
        icon: <ShieldIcon />,
        disabled: true,
        isSoon: true,
      },
      {
        label: "Event webhooks",
        icon: <ServerIcon />,
        disabled: true,
        isSoon: true,
      },
    ],
  },
  {
    label: "Inbound",
    icon: <InboxIcon />,
    children: [
      { label: "Mailboxes", icon: <InboxIcon />, view: "mailboxes" },
      {
        label: "Mailbox administration",
        icon: <SettingsIcon />,
        view: "mailbox-admin",
      },
      {
        label: "Routing rules",
        icon: <SettingsIcon />,
        disabled: true,
        isSoon: true,
      },
    ],
  },
  {
    label: "Developers",
    icon: <ServerIcon />,
    children: [
      { label: "API keys", icon: <KeyIcon />, view: "api-keys" },
      {
        label: "API reference",
        icon: <ServerIcon />,
        disabled: true,
        isSoon: true,
      },
      {
        label: "Webhooks",
        icon: <ServerIcon />,
        disabled: true,
        isSoon: true,
      },
    ],
  },
  {
    label: "Account",
    icon: <UserIcon />,
    children: [
      { label: "Credentials", icon: <UserIcon />, view: "credentials" },
      {
        label: "SMTP credentials",
        icon: <KeyIcon />,
        view: "smtp-credentials",
      },
      {
        label: "Team & access",
        icon: <UsersIcon />,
        disabled: true,
        isSoon: true,
      },
      {
        label: "Usage & billing",
        icon: <ServerIcon />,
        disabled: true,
        isSoon: true,
      },
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
  mobile,
  mobileOpen,
  onMobileClose,
  closeButtonRef,
  connected,
  loading,
  connectionError,
  principal,
  tenant,
}: DashboardSidebarProps) {
  const sidebarRef = useRef<HTMLElement>(null);
  const [expanded, setExpanded] = useState<string[]>(
    () => groupsForView(navigation, view) ?? [],
  );

  useLayoutEffect(() => {
    if (mobile && mobileOpen) {
      closeButtonRef.current?.focus();
    }
  }, [closeButtonRef, mobile, mobileOpen]);

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
            disabled={item.disabled}
            onClick={() => {
              if (item.disabled) return;
              if (hasChildren) {
                toggle(item.label);
              } else if (item.view) {
                onViewChange(item.view);
                onMobileClose();
              }
            }}
            title={item.disabled ? `${item.label} — coming later` : undefined}
            type="button"
          >
            <span className="nav-icon" aria-hidden="true">
              {item.icon}
            </span>
            <span className="nav-label">{item.label}</span>
            {(item.disabled || item.isSoon) && (
              <span className="nav-coming-soon">Soon</span>
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

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (!mobileOpen) return;
    if (event.key !== "Tab" || !sidebarRef.current) return;

    const focusable = sidebarRef.current.querySelectorAll<HTMLElement>(
      'button:not(:disabled), a[href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
    );
    const first = focusable.item(0);
    const last = focusable.item(focusable.length - 1);
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  }

  return (
    <aside
      id="dashboard-sidebar"
      ref={sidebarRef}
      aria-hidden={mobile && !mobileOpen ? true : undefined}
      aria-label="Dashboard navigation"
      aria-modal={mobile && mobileOpen ? true : undefined}
      className={`sidebar ${mobileOpen ? "sidebar-open" : ""}`}
      inert={mobile && !mobileOpen}
      onKeyDown={handleKeyDown}
      role={mobile && mobileOpen ? "dialog" : "complementary"}
    >
      <div className="sidebar-heading">
        <button
          aria-label={`${brandName} home`}
          className="brand"
          onClick={() => {
            onViewChange("home");
            onMobileClose();
          }}
          type="button"
        >
          <span className="brand-mark" aria-hidden="true">
            S
          </span>
          <span>
            <strong>{brandName}</strong>
            <small>Email platform</small>
          </span>
        </button>
        <button
          aria-label="Close navigation"
          className="drawer-close"
          onClick={onMobileClose}
          ref={closeButtonRef}
          type="button"
        >
          <XIcon />
        </button>
      </div>

      <nav className="navigation" aria-label="Main navigation">
        {renderItems(navigation)}
      </nav>

      <div className="sidebar-bottom">
        <div
          aria-live="polite"
          className={`connection ${connected ? "online" : ""}`}
          role="status"
        >
          <span className="connection-dot" aria-hidden="true" />
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
          <span className="avatar" aria-hidden="true">
            {(principal?.subject?.[0] ?? "U").toUpperCase()}
          </span>
          <span>
            <strong>{principal?.role ?? "Not connected"}</strong>
            <small>
              {tenant?.name ?? principal?.tenant?.name ?? "Workspace"}
            </small>
          </span>
        </div>
      </div>
    </aside>
  );
}
