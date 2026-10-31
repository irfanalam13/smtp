import { requestJson } from "@/lib/api/json-client";

const publicContext = { token: "", tenantId: "" };

export type AuthCapabilities = {
  oidc_enabled: boolean;
  password_login_enabled: boolean;
  signup_enabled: boolean;
  password_reset_enabled: boolean;
};

type AuthRequestBody =
  | { email: string; password: string; organization_name: string }
  | { email: string; password: string; tenant_id?: string }
  | { email: string }
  | { token: string }
  | { token: string; new_password: string };

type AccountEmailAction = {
  status: "accepted" | "verification_pending";
  message: string;
  tenant_id?: string;
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
  return requestJson<AuthCapabilities>(
    "/v1/auth/providers",
    publicContext,
    { signal },
  );
}

export function signUp(input: {
  email: string;
  password: string;
  organization_name: string;
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
