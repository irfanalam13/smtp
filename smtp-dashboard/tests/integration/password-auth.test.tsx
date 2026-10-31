import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PasswordLogin } from "../../src/features/auth/components/password-login";
import { LogoutButton } from "../../src/features/auth/components/logout-button";
import { ResetPasswordForm } from "../../src/features/auth/components/reset-password-form";
import { SignupForm } from "../../src/features/auth/components/signup-form";

const { replace, refresh } = vi.hoisted(() => ({
  replace: vi.fn(),
  refresh: vi.fn(),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace, refresh, push: vi.fn() }),
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  replace.mockReset();
  refresh.mockReset();
});

describe("password authentication forms", () => {
  it("submits email login to the API and navigates after a session is created", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({
          oidc_enabled: false,
          password_login_enabled: true,
          signup_enabled: true,
          password_reset_enabled: true,
        })),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({
          status: "authenticated",
          user_id: "user-1",
          tenant_id: "tenant-1",
          expires_at: "2030-01-01T00:00:00Z",
        })),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(<PasswordLogin />);

    fireEvent.change(await screen.findByLabelText("Email address"), {
      target: { value: "owner@example.test" },
    });
    fireEvent.change(screen.getByLabelText(/^Password/), {
      target: { value: "a secure password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(replace).toHaveBeenCalledWith("/home");
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const [url, init] = fetchMock.mock.calls[1];
    expect(url).toBe("/api/backend/v1/auth/login");
    expect(JSON.parse(String(init?.body))).toEqual({
      email: "owner@example.test",
      password: "a secure password",
    });
    expect(init?.credentials).toBe("same-origin");
  });

  it("submits signup and exposes the verification resend action", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({
          oidc_enabled: false,
          password_login_enabled: true,
          signup_enabled: true,
          password_reset_enabled: true,
        })),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({
          status: "verification_pending",
          message: "Check your email for a verification link.",
        })),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(<SignupForm />);

    fireEvent.change(await screen.findByLabelText("Work email"), {
      target: { value: "owner@example.test" },
    });
    fireEvent.change(screen.getByLabelText("Organization name"), {
      target: { value: "Example Org" },
    });
    fireEvent.change(screen.getByLabelText(/^Password/), {
      target: { value: "a secure password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Check your email for a verification link.",
    );
    expect(
      screen.getByRole("button", { name: "Resend verification email" }),
    ).toBeInTheDocument();
    const [url, init] = fetchMock.mock.calls[1];
    expect(url).toBe("/api/backend/v1/auth/signup");
    expect(JSON.parse(String(init?.body))).toEqual({
      email: "owner@example.test",
      password: "a secure password",
      organization_name: "Example Org",
    });
  });

  it("uses the logout endpoint and returns to sign in", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ status: "logged_out" })),
    );
    vi.stubGlobal("fetch", fetchMock);
    render(<LogoutButton />);

    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    await waitFor(() => expect(replace).toHaveBeenCalledWith("/login"));
    expect(fetchMock.mock.calls[0][0]).toBe("/api/backend/v1/auth/logout");
    expect(fetchMock.mock.calls[0][1]?.credentials).toBe("same-origin");
  });

  it("submits a single-use reset token and removes it from the URL on success", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock
      .mockResolvedValueOnce(
        new Response(JSON.stringify({
          oidc_enabled: false,
          password_login_enabled: true,
          signup_enabled: true,
          password_reset_enabled: true,
        })),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: "password_reset" })),
      );
    vi.stubGlobal("fetch", fetchMock);
    render(<ResetPasswordForm token="single-use-reset-token" />);

    fireEvent.change(await screen.findByLabelText("New password"), {
      target: { value: "a secure password" },
    });
    fireEvent.change(screen.getByLabelText("Confirm new password"), {
      target: { value: "a secure password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Update password" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Your password has been changed",
    );
    expect(replace).toHaveBeenCalledWith("/reset-password");
    const [url, init] = fetchMock.mock.calls[1];
    expect(url).toBe("/api/backend/v1/auth/password/reset");
    expect(JSON.parse(String(init?.body))).toEqual({
      token: "single-use-reset-token",
      new_password: "a secure password",
    });
  });
});
