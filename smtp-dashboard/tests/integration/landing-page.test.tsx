import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { LandingPage } from "@/features/landing/landing-page";

afterEach(() => {
  cleanup();
});

describe("LandingPage Component", () => {
  it("renders the hero heading and project brief", () => {
    render(<LandingPage />);

    expect(
      screen.getByRole("heading", {
        name: /High-Assurance SMTP Cloud Infrastructure/i,
      }),
    ).toBeInTheDocument();

    expect(
      screen.getByRole("heading", {
        name: /Why We Engineered the SMTP Platform/i,
      }),
    ).toBeInTheDocument();

    expect(
      screen.getByText(/Decoupled Service Boundaries/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/Strict Tenant Isolation/i)).toBeInTheDocument();
    expect(screen.getByText(/Durable Leased Queueing/i)).toBeInTheDocument();
  });

  it("renders all critical auth redirection links and buttons", () => {
    render(<LandingPage />);

    // Check login buttons/links
    const loginLinks = screen.getAllByRole("link", { name: /Sign In/i });
    expect(loginLinks.length).toBeGreaterThan(0);
    expect(
      loginLinks.some((link) => link.getAttribute("href") === "/login"),
    ).toBe(true);

    // Check signup / register links
    const signupLinks = screen.getAllByRole("link", {
      name: /Create Account|Register Organization|Register Account/i,
    });
    expect(signupLinks.length).toBeGreaterThan(0);
    expect(
      signupLinks.some((link) => link.getAttribute("href") === "/signup"),
    ).toBe(true);

    // Check console link
    const consoleLinks = screen.getAllByRole("link", {
      name: /Console|Open Console|Open Control Plane/i,
    });
    expect(consoleLinks.length).toBeGreaterThan(0);
    expect(
      consoleLinks.some((link) => link.getAttribute("href") === "/home"),
    ).toBe(true);

    // Check password reset & email verification links
    const verifyLinks = screen.getAllByRole("link", { name: /Verify Email/i });
    expect(
      verifyLinks.some((link) => link.getAttribute("href") === "/verify-email"),
    ).toBe(true);

    const forgotLinks = screen.getAllByRole("link", {
      name: /Forgot Password|Reset Password/i,
    });
    expect(
      forgotLinks.some(
        (link) => link.getAttribute("href") === "/forgot-password",
      ),
    ).toBe(true);
  });

  it("switches code snippet tabs on click", () => {
    render(<LandingPage />);

    // Default tab is Python
    expect(screen.getByText(/submission-client\.py/i)).toBeInTheDocument();

    // Click cURL tab
    const curlTab = screen.getByRole("button", { name: "CURL" });
    fireEvent.click(curlTab);
    expect(screen.getByText(/submission-client\.sh/i)).toBeInTheDocument();
    expect(
      screen.getByText(/http:\/\/127\.0\.0\.1:8080\/v1\/submissions/i),
    ).toBeInTheDocument();

    // Click Node tab
    const nodeTab = screen.getByRole("button", { name: "NODE" });
    fireEvent.click(nodeTab);
    expect(screen.getByText(/submission-client\.ts/i)).toBeInTheDocument();
    expect(
      screen.getByText(/nodemailer\.createTransport/i),
    ).toBeInTheDocument();
  });

  it("displays monorepo architecture component cards", () => {
    render(<LandingPage />);

    expect(screen.getAllByText("smtp-session").length).toBeGreaterThan(0);
    expect(screen.getAllByText("smtp-queue").length).toBeGreaterThan(0);
    expect(screen.getAllByText("smtp-delivery").length).toBeGreaterThan(0);
    expect(screen.getAllByText("mailbox-service").length).toBeGreaterThan(0);
    expect(screen.getAllByText("smtp-api").length).toBeGreaterThan(0);
    expect(screen.getAllByText("smtp-auth").length).toBeGreaterThan(0);
  });
});
