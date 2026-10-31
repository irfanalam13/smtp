export type RequestContext = {
  token: string;
  tenantId: string;
};

export type ApiErrorCategory =
  | "validation"
  | "authentication"
  | "permission"
  | "not_found"
  | "conflict"
  | "payload_too_large"
  | "rate_limited"
  | "server"
  | "http"
  | "network"
  | "timeout"
  | "invalid_response";

type APIErrorPayload = {
  error?: {
    code?: unknown;
    message?: unknown;
    request_id?: unknown;
  };
};

export class ApiError extends Error {
  readonly category: ApiErrorCategory;
  readonly status: number | null;
  readonly code: string | null;
  readonly requestId: string | null;
  readonly retryAfterSeconds: number | null;

  constructor(options: {
    message: string;
    category: ApiErrorCategory;
    status?: number | null;
    code?: string | null;
    requestId?: string | null;
    retryAfterSeconds?: number | null;
  }) {
    super(options.message);
    this.name = "ApiError";
    this.category = options.category;
    this.status = options.status ?? null;
    this.code = options.code ?? null;
    this.requestId = options.requestId ?? null;
    this.retryAfterSeconds = options.retryAfterSeconds ?? null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorPayload(value: unknown): APIErrorPayload {
  if (!isRecord(value) || !isRecord(value.error)) {
    return {};
  }
  return { error: value.error };
}

function responseErrorMessage(payload: unknown, status: number): string {
  const message = errorPayload(payload).error?.message;
  if (typeof message === "string" && message.trim()) {
    return message;
  }

  const statusMessages: Partial<Record<number, string>> = {
    400: "The request contains invalid values.",
    401: "Authentication is required or has expired.",
    403: "Your account does not have permission for this action.",
    404: "The requested resource was not found.",
    409: "The request conflicts with the current resource state.",
    413: "The request exceeds the API size limit.",
    429: "The API rate limit was reached. Try again later.",
  };
  return (
    statusMessages[status] ??
    (status >= 500
      ? "The API is temporarily unavailable."
      : `The API returned HTTP ${status}.`)
  );
}

function categoryForStatus(status: number): ApiErrorCategory {
  if (status === 400) return "validation";
  if (status === 401) return "authentication";
  if (status === 403) return "permission";
  if (status === 404) return "not_found";
  if (status === 409) return "conflict";
  if (status === 413) return "payload_too_large";
  if (status === 429) return "rate_limited";
  if (status >= 500) return "server";
  return "http";
}

function retryAfterSeconds(response: Response): number | null {
  const value = response.headers.get("Retry-After");
  if (!value) return null;

  const seconds = Number(value);
  if (Number.isFinite(seconds)) {
    return Math.max(0, seconds);
  }

  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp)
    ? null
    : Math.max(0, Math.ceil((timestamp - Date.now()) / 1000));
}

function backendPath(path: string): string {
  const pathname = path.split(/[?#]/, 1)[0];
  const hasUnsafeSegment = pathname.split("/").some((segment) => {
    try {
      const decoded = decodeURIComponent(segment);
      return (
        decoded === "." ||
        decoded === ".." ||
        decoded.includes("/") ||
        decoded.includes("\\")
      );
    } catch {
      return true;
    }
  });
  if (
    !path.startsWith("/") ||
    path.startsWith("//") ||
    path.includes("\\") ||
    path.includes("://") ||
    hasUnsafeSegment
  ) {
    throw new TypeError("API requests must use a relative backend path.");
  }
  return `/api/backend${path}`;
}

function requestHeaders(context: RequestContext, init?: RequestInit): Headers {
  const headers = new Headers(init?.headers);
  headers.set("Accept", "application/json");
  if (init?.body != null && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const token = context.token.trim();
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
    headers.delete("X-Dev-Subject");
  } else {
    headers.delete("Authorization");
    if (process.env.NODE_ENV === "development") {
      headers.set("X-Dev-Subject", "dashboard-local-user");
    } else {
      headers.delete("X-Dev-Subject");
    }
  }

  const tenantId = context.tenantId.trim();
  if (tenantId) {
    headers.set("X-Tenant-ID", tenantId);
  } else {
    headers.delete("X-Tenant-ID");
  }
  const csrfToken = csrfCookieValue();
  if (csrfToken) {
    headers.set("X-CSRF-Token", csrfToken);
  }
  return headers;
}

function csrfCookieValue(): string | null {
  if (typeof document === "undefined") {
    return null;
  }
  const configuredName = process.env.NEXT_PUBLIC_AUTH_CSRF_COOKIE_NAME?.trim();
  const names = [
    configuredName,
    "__Host-smtp_csrf",
    "smtp_csrf",
  ].filter((name): name is string => Boolean(name));
  const cookies = document.cookie.split(";");
  for (const name of names) {
    const cookie = cookies.find((item) => item.trim().startsWith(`${name}=`));
    if (cookie) {
      return cookie.trim().slice(name.length + 1);
    }
  }
  return null;
}

function apiErrorFromPayload(payload: unknown, response: Response): ApiError {
  const body = errorPayload(payload).error;
  const code = typeof body?.code === "string" ? body.code : null;
  const bodyRequestId =
    typeof body?.request_id === "string" ? body.request_id : null;
  return new ApiError({
    message: responseErrorMessage(payload, response.status),
    category: categoryForStatus(response.status),
    status: response.status,
    code,
    requestId: response.headers.get("X-Request-ID") ?? bodyRequestId,
    retryAfterSeconds: retryAfterSeconds(response),
  });
}

function isAbortError(reason: unknown): boolean {
  return isRecord(reason) && "name" in reason && reason.name === "AbortError";
}

async function requestWithBody<T>(
  path: string,
  context: RequestContext,
  consume: (response: Response) => Promise<T>,
  init?: RequestInit,
): Promise<T> {
  const url = backendPath(path);
  const controller = new AbortController();
  const callerSignal = init?.signal;
  let timedOut = false;
  const abortFromCaller = () => controller.abort();
  if (callerSignal?.aborted) {
    controller.abort();
  } else {
    callerSignal?.addEventListener("abort", abortFromCaller, { once: true });
  }
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, 30_000);

  try {
    const response = await fetch(url, {
      ...init,
      credentials: "same-origin",
      signal: controller.signal,
      headers: requestHeaders(context, init),
      cache: "no-store",
    });

    if (!response.ok) {
      const responseText = await response.text();
      let payload: unknown = null;
      if (responseText.trim()) {
        try {
          payload = JSON.parse(responseText) as unknown;
        } catch {
          payload = null;
        }
      }
      throw apiErrorFromPayload(payload, response);
    }
    return await consume(response);
  } catch (reason) {
    if (reason instanceof ApiError) {
      throw reason;
    }
    if (timedOut) {
      throw new ApiError({
        message: "The API request timed out. Try again.",
        category: "timeout",
      });
    }
    if (callerSignal?.aborted || isAbortError(reason)) {
      throw reason;
    }
    throw new ApiError({
      message: "The dashboard could not reach the API.",
      category: "network",
    });
  } finally {
    clearTimeout(timeout);
    callerSignal?.removeEventListener("abort", abortFromCaller);
  }
}

export function requestJson<T>(
  path: string,
  context: RequestContext,
  init?: RequestInit,
): Promise<T> {
  return requestWithBody(
    path,
    context,
    async (response) => {
      if (response.status === 204) return undefined as T;
      const responseText = await response.text();
      if (!responseText.trim()) {
        throw new ApiError({
          message: "The API returned an empty or invalid JSON response.",
          category: "invalid_response",
          status: response.status,
          requestId: response.headers.get("X-Request-ID"),
        });
      }
      try {
        return JSON.parse(responseText) as T;
      } catch {
        throw new ApiError({
          message: "The API returned an empty or invalid JSON response.",
          category: "invalid_response",
          status: response.status,
          requestId: response.headers.get("X-Request-ID"),
        });
      }
    },
    init,
  );
}

export function requestText(
  path: string,
  context: RequestContext,
  init?: RequestInit,
): Promise<string> {
  return requestWithBody(path, context, (response) => response.text(), init);
}

export function requestBlob(
  path: string,
  context: RequestContext,
  init?: RequestInit,
): Promise<Blob> {
  return requestWithBody(path, context, (response) => response.blob(), init);
}
