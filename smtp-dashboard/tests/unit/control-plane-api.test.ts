import { beforeEach, describe, expect, it, vi } from "vitest";
import { requestJson } from "../../src/lib/api/json-client";
import {
  createApiKey,
  disableDomain,
  getSendingConfig,
  listMailboxMessages,
  listSubmissions,
  submitMessage,
} from "../../src/features/control-plane/services/control-plane-api";

vi.mock("../../src/lib/api/json-client", () => ({
  requestJson: vi.fn(),
}));

const context = { token: "secret", tenantId: "tenant-id" };
const request = vi.mocked(requestJson);

beforeEach(() => {
  request.mockReset();
  request.mockResolvedValue({ items: [] } as never);
});

describe("control-plane API service", () => {
  it("uses bounded list queries and encodes resource identifiers", async () => {
    await listSubmissions(context);
    expect(request).toHaveBeenLastCalledWith(
      "/v1/submissions?limit=20",
      context,
      { signal: undefined },
    );

    await listMailboxMessages(context, "mailbox id");
    expect(request).toHaveBeenLastCalledWith(
      "/v1/mailboxes/mailbox%20id/messages?limit=50",
      context,
      { signal: undefined },
    );
  });

  it("loads SMTP readiness without requesting relay credentials", async () => {
    await getSendingConfig(context);
    expect(request).toHaveBeenLastCalledWith("/v1/sending/config", context, {
      signal: undefined,
    });
  });

  it("sends the selected scopes and expiry when creating an API key", async () => {
    const keyRequest = {
      name: "mailer",
      scopes: ["delivery:write"] as const,
      expires_at: "2030-01-01T00:00:00.000Z",
    };
    await createApiKey(context, {
      ...keyRequest,
      scopes: [...keyRequest.scopes],
    });

    expect(request).toHaveBeenLastCalledWith("/v1/api-keys", context, {
      method: "POST",
      body: JSON.stringify(keyRequest),
    });
  });

  it("encodes domain IDs when disabling a sender domain", async () => {
    await disableDomain(context, "domain id");

    expect(request).toHaveBeenLastCalledWith(
      "/v1/domains/domain%20id",
      context,
      { method: "DELETE" },
    );
  });

  it("sends submission idempotency in the header and plain-text payload", async () => {
    const message = {
      from: "sender@example.com",
      to: ["recipient@example.net"],
      cc: [],
      bcc: [],
      subject: "Hello",
      text: "Plain text",
    };
    await submitMessage(context, "stable-retry-key", message);

    expect(request).toHaveBeenLastCalledWith("/v1/submissions", context, {
      method: "POST",
      headers: { "Idempotency-Key": "stable-retry-key" },
      body: JSON.stringify(message),
    });
  });
});
