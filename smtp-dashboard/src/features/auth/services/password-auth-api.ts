import { ApiError, requestJson } from "@/lib/api/json-client";

const publicContext = { token: "", tenantId: "" };

export type AuthCapabilities = {
  oidc_enabled: boolean;
  google_enabled?: boolean;
  github_enabled?: boolean;
  password_login_enabled: boolean;
  signup_enabled: boolean;
  password_reset_enabled: boolean;
};

type AuthRequestBody =
  | {
      first_name: string;
      last_name: string;
      email: string;
      password: string;
      confirm_password: string;
      terms_accepted: boolean;
      phone_number?: string;
      country?: string;
      organization_name?: string;
    }
  | { email: string; password: string; tenant_id?: string }
  | { email: string }
  | { token: string }
  | { token: string; new_password: string };

type AccountEmailAction = {
  status: "accepted" | "verification_pending" | "verified";
  message: string;
  tenant_id?: string;
  verification_url?: string;
};

function post<T>(path: string, body: AuthRequestBody): Promise<T> {
  return requestJson<T>(path, publicContext, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function getAuthCapabilities(
  signal?: AbortSignal,
): Promise<AuthCapabilities> {
  return requestCapabilitiesWithRetry(signal);
}

async function requestCapabilitiesWithRetry(
  signal?: AbortSignal,
): Promise<AuthCapabilities> {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    try {
      return await requestJson<AuthCapabilities>(
        "/v1/auth/providers",
        publicContext,
        { signal },
      );
    } catch (reason) {
      const retryable =
        reason instanceof ApiError &&
        (reason.category === "network" ||
          reason.category === "timeout" ||
          reason.category === "server");
      if (!retryable || attempt === 1 || signal?.aborted) {
        throw reason;
      }
      await new Promise<void>((resolve, reject) => {
        const timeout = setTimeout(resolve, 300);
        signal?.addEventListener(
          "abort",
          () => {
            clearTimeout(timeout);
            reject(signal.reason);
          },
          { once: true },
        );
      });
    }
  }
  throw new Error("Authentication capabilities could not be loaded.");
}

export function signUp(input: {
  first_name: string;
  last_name: string;
  email: string;
  password: string;
  confirm_password: string;
  terms_accepted: boolean;
  phone_number?: string;
  country?: string;
  organization_name?: string;
}): Promise<AccountEmailAction> {
  return post("/v1/auth/signup", input);
}

export function passwordLogin(input: {
  email: string;
  password: string;
  tenant_id?: string;
}): Promise<{
  status: "authenticated";
  user_id: string;
  tenant_id: string;
  expires_at: string;
  access_token?: string;
  token_type?: "Bearer";
  access_token_expires_at?: string;
}> {
  return post("/v1/auth/login", input);
}

export function verifyEmail(token: string): Promise<{ status: "verified" }> {
  return post("/v1/auth/email/verify", { token });
}

export function resendVerification(email: string): Promise<AccountEmailAction> {
  return post("/v1/auth/email/resend-verification", { email });
}

export function requestPasswordReset(email: string): Promise<AccountEmailAction> {
  return post("/v1/auth/password/forgot", { email });
}

export function resetPassword(
  token: string,
  newPassword: string,
): Promise<{ status: "password_reset" }> {
  return post("/v1/auth/password/reset", {
    token,
    new_password: newPassword,
  });
}
