import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, requestJson } from "../../src/lib/api/json-client";

const context = { token: "user-bound-token", tenantId: "tenant-123" };

afterEach(() => {
  document.cookie = "smtp_csrf=; Max-Age=0; path=/";
  document.cookie = "__Host-smtp_csrf=; Max-Age=0; path=/";
  vi.unstubAllGlobals();
});

describe("requestJson", () => {
  it("uses the same-origin proxy and required request headers", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ accepted: true }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      requestJson<{ accepted: boolean }>("/v1/submissions", context, {
        method: "POST",
        headers: { "Idempotency-Key": "retry-123" },
        body: JSON.stringify({ text: "hello" }),
      }),
    ).resolves.toEqual({ accepted: true });

    expect(fetchMock).toHaveBeenCalledOnce();
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/backend/v1/submissions");
    expect(init?.cache).toBe("no-store");
    const headers = new Headers(init?.headers);
    expect(headers.get("Authorization")).toBe("Bearer user-bound-token");
    expect(headers.get("X-Tenant-ID")).toBe("tenant-123");
    expect(headers.get("Idempotency-Key")).toBe("retry-123");
    expect(headers.get("Accept")).toBe("application/json");
    expect(headers.get("Content-Type")).toBe("application/json");
    expect(headers.has("X-Dev-Subject")).toBe(false);
  });

  it("uses the readable CSRF cookie only as a request header", async () => {
    document.cookie = "smtp_csrf=csrf-token-for-test; path=/";
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ accepted: true }), { status: 200 }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await requestJson("/v1/auth/logout-all", { token: "", tenantId: "" }, {
      method: "POST",
    });

    const [, init] = fetchMock.mock.calls[0];
    expect(new Headers(init?.headers).get("X-CSRF-Token")).toBe(
      "csrf-token-for-test",
    );
    expect(init?.credentials).toBe("same-origin");
    expect(new Headers(init?.headers).has("Authorization")).toBe(false);
  });

  it("rejects paths that could escape the backend proxy", async () => {
    await expect(requestJson("/../health/ready", context)).rejects.toThrow(
      "relative backend path",
    );
    await expect(
      requestJson("//attacker.invalid/path", context),
    ).rejects.toThrow("relative backend path");
    await expect(
      requestJson("/v1/%2e%2e/health/ready", context),
    ).rejects.toThrow("relative backend path");
  });

  it("maps API permission errors and retry metadata", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(
      new Response(
        JSON.stringify({
          error: {
            code: "permission_denied",
            message: "Missing required scope.",
            request_id: "body-request-id",
          },
        }),
        {
          status: 403,
          headers: {
            "Retry-After": "12",
            "X-Request-ID": "header-request-id",
          },
        },
      ),
    );
    vi.stubGlobal("fetch", fetchMock);

    const request = requestJson("/v1/api-keys", context);
    await expect(request).rejects.toBeInstanceOf(ApiError);
    await expect(request).rejects.toMatchObject({
      category: "permission",
      status: 403,
      code: "permission_denied",
      message: "Missing required scope.",
      requestId: "header-request-id",
      retryAfterSeconds: 12,
    });
  });

  it("surfaces invalid success payloads and preserves caller cancellation", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    fetchMock.mockResolvedValue(new Response("not-json", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(requestJson("/v1/me", context)).rejects.toMatchObject({
      category: "invalid_response",
    });

    const controller = new AbortController();
    controller.abort();
    fetchMock.mockRejectedValue(new DOMException("Aborted", "AbortError"));
    await expect(
      requestJson("/v1/me", context, { signal: controller.signal }),
    ).rejects.toHaveProperty("name", "AbortError");
  });
});
