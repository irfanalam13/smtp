import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { OIDCSignIn } from "../../src/features/auth/components/oidc-sign-in";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.history.replaceState({}, "", "/");
});

describe("OIDC sign-in", () => {
  it("shows a real same-origin sign-in link only when OIDC is enabled", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ oidc_enabled: true }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    render(<OIDCSignIn />);

    const signIn = await screen.findByRole("link", {
      name: "Continue with organization SSO",
    });
    expect(signIn.getAttribute("href")).toBe(
      "/api/backend/v1/auth/oidc/start",
    );

    fireEvent.change(screen.getByLabelText("Organization ID"), {
      target: { value: "123e4567-e89b-12d3-a456-426614174000" },
    });
    await waitFor(() => {
      expect(signIn.getAttribute("href")).toContain(
        "?tenant_id=123e4567-e89b-12d3-a456-426614174000",
      );
    });
  });

  it("does not render a provider button when the backend has no configured provider", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ oidc_enabled: false }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    render(<OIDCSignIn />);

    expect(
      await screen.findByText(/Enterprise sign-in is not configured/),
    ).toBeTruthy();
    expect(
      screen.queryByRole("link", { name: "Continue with organization SSO" }),
    ).toBeNull();
  });

  it("announces provider callback failures accessibly", async () => {
    window.history.replaceState({}, "", "/login?error=oidc_failed");
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ oidc_enabled: false }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);
    render(<OIDCSignIn />);

    const alert = await screen.findByText(
      "Your identity provider could not complete sign-in. Please try again.",
    );
    expect(alert.getAttribute("role")).toBe("alert");
  });
});
