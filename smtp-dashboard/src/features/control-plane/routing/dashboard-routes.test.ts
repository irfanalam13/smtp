import { describe, expect, it } from "vitest";
import {
  dashboardPathForView,
  dashboardViewForPath,
  isDashboardPath,
} from "./dashboard-routes";
import type { DashboardView } from "../types/control-plane";

const views: DashboardView[] = [
  "home",
  "send",
  "activity",
  "domains",
  "mailboxes",
  "mailbox-admin",
  "api-keys",
  "smtp-credentials",
  "credentials",
  "statistics",
  "templates",
  "sms",
  "suppressions",
  "event-webhooks",
  "routing-rules",
  "api-reference",
  "webhooks",
  "team",
  "billing",
];

describe("dashboard route mapping", () => {
  it("round-trips every dashboard view through its feature URL", () => {
    for (const view of views) {
      const path = dashboardPathForView(view);
      expect(dashboardViewForPath(path)).toBe(view);
      expect(isDashboardPath(path)).toBe(true);
    }
  });

  it("keeps the root URL as a home alias and rejects unknown URLs", () => {
    expect(dashboardViewForPath("/")).toBe("home");
    expect(isDashboardPath("/")).toBe(true);
    expect(dashboardViewForPath("/unknown/feature")).toBeNull();
    expect(isDashboardPath("/unknown/feature")).toBe(false);
  });
});
