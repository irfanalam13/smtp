import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ControlPlaneDashboard from "../../src/features/control-plane/components/shell/control-plane-dashboard";

const dashboardMock = vi.hoisted(() => ({
  view: "home",
  setView: () => undefined,
  connected: false,
  loading: false,
  connectionError: "API unavailable",
  principal: null,
  tenant: null,
  apiToken: "",
  actionError: "",
  actionMessage: "",
  setActionError: () => undefined,
  setActionMessage: () => undefined,
  domains: [],
  verifiedDomains: 0,
  keys: [],
  submissions: [],
  usage: null,
  readiness: null,
  loadErrors: {},
  load: async () => undefined,
}));

vi.mock(
  "../../src/features/control-plane/hooks/use-control-plane-dashboard",
  () => ({
    useControlPlaneDashboard: () => dashboardMock,
  }),
);

beforeEach(() => {
  vi.stubGlobal(
    "matchMedia",
    vi.fn().mockReturnValue({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }),
  );
  document.body.style.overflow = "";
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.style.overflow = "";
});

describe("control-plane dashboard shell", () => {
  it("opens the mobile navigation as a dialog and restores focus on Escape", async () => {
    render(<ControlPlaneDashboard />);

    const menuButton = screen.getByRole("button", {
      name: "Open dashboard menu",
    });
    fireEvent.click(menuButton);

    const navigation = screen.getByRole("dialog", {
      name: "Dashboard navigation",
    });
    expect(menuButton).toHaveAttribute("aria-expanded", "true");
    expect(
      within(navigation).getByRole("button", { name: "Close navigation" }),
    ).toHaveFocus();
    expect(document.body.style.overflow).toBe("hidden");

    const firstItem = within(navigation).getByRole("button", {
      name: "SMTP Platform home",
    });
    const lastItem = within(navigation).getByRole("button", {
      name: "Account",
    });
    firstItem.focus();
    fireEvent.keyDown(firstItem, { key: "Tab", shiftKey: true });
    expect(lastItem).toHaveFocus();
    fireEvent.keyDown(lastItem, { key: "Tab" });
    expect(firstItem).toHaveFocus();

    const closeButton = within(navigation).getByRole("button", {
      name: "Close navigation",
    });
    closeButton.focus();
    fireEvent.keyDown(closeButton, { key: "Escape" });

    await waitFor(() => expect(menuButton).toHaveFocus());
    expect(menuButton).toHaveAttribute("aria-expanded", "false");
    expect(document.body.style.overflow).toBe("");
  });
});
