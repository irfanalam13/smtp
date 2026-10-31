import type { DashboardView } from "../types/control-plane";

const paths: Record<DashboardView, string> = {
  home: "/home",
  send: "/transaction/email/send",
  activity: "/transaction/email/activity",
  domains: "/deliverability/domains",
  mailboxes: "/inbound/mailboxes",
  "mailbox-admin": "/inbound/mailboxes/manage",
  "api-keys": "/developers/api-keys",
  "smtp-credentials": "/account/smtp-credentials",
  credentials: "/account/credentials",
  statistics: "/transaction/email/statistics",
  templates: "/transaction/email/templates",
  sms: "/transaction/sms",
  suppressions: "/deliverability/suppressions",
  "event-webhooks": "/deliverability/event-webhooks",
  "routing-rules": "/inbound/routing-rules",
  "api-reference": "/developers/api-reference",
  webhooks: "/developers/webhooks",
  team: "/account/team",
  billing: "/account/billing",
};

const viewsByPath = new Map(
  Object.entries(paths).map(([view, path]) => [path, view as DashboardView]),
);

export function dashboardPathForView(view: DashboardView): string {
  return paths[view];
}

export function dashboardViewForPath(pathname: string): DashboardView | null {
  if (pathname === "/") {
    return "home";
  }
  return viewsByPath.get(pathname) ?? null;
}

export function isDashboardPath(pathname: string): boolean {
  return pathname === "/" || viewsByPath.has(pathname);
}
