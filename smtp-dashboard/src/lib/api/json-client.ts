export type RequestContext = {
  token: string;
  tenantId: string;
};

function apiErrorMessage(payload: unknown, status: number): string {
  if (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof payload.error === "object" &&
    payload.error !== null &&
    "message" in payload.error &&
    typeof payload.error.message === "string"
  ) {
    return payload.error.message;
  }
  return `The API returned HTTP ${status}`;
}

export async function requestJson<T>(
  path: string,
  context: RequestContext,
  init?: RequestInit,
): Promise<T> {
  const headers = new Headers(init?.headers);
  headers.set("Accept", "application/json");
  if (init?.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  if (context.token) {
    headers.set("Authorization", `Bearer ${context.token}`);
  } else if (process.env.NODE_ENV === "development") {
    headers.set("X-Dev-Subject", "dashboard-local-user");
  }
  if (context.tenantId) {
    headers.set("X-Tenant-ID", context.tenantId);
  }

  const response = await fetch(`/api/backend${path}`, {
    ...init,
    headers,
    cache: "no-store",
  });
  const payload: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    throw new Error(apiErrorMessage(payload, response.status));
  }
  if (payload === null) {
    throw new Error("The API returned an empty or invalid JSON response");
  }
  return payload as T;
}
