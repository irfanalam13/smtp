import { requestJson } from "@/lib/api/json-client";
import type { RequestContext } from "@/lib/api/json-client";
import type {
  ApiKey,
  APIKeyScope,
  CreatedApiKey,
  Domain,
  DomainCheck,
  DomainReadiness,
  ListResponse,
  Mailbox,
  Message,
  Principal,
  Submission,
  SubmissionAccepted,
  SubmissionAttempt,
  SendingConfig,
  Tenant,
  Usage,
  VerificationChallenge,
} from "../types/control-plane";

function listPath(path: string, limit: number): string {
  const query = new URLSearchParams({ limit: String(limit) });
  return `${path}?${query.toString()}`;
}

export function getReadiness(context: RequestContext, signal?: AbortSignal) {
  return requestJson<{ status: "ready" | "not_ready" }>(
    "/health/ready",
    context,
    { signal },
  );
}

export function getPrincipal(context: RequestContext, signal?: AbortSignal) {
  return requestJson<Principal>("/v1/me", context, { signal });
}

export function getTenant(context: RequestContext, signal?: AbortSignal) {
  return requestJson<Tenant>("/v1/tenants/current", context, { signal });
}

export function listDomains(context: RequestContext, signal?: AbortSignal) {
  return requestJson<ListResponse<Domain>>(
    listPath("/v1/domains", 50),
    context,
    { signal },
  );
}

export function getDomainChecks(
  context: RequestContext,
  domainId: string,
  signal?: AbortSignal,
) {
  return requestJson<DomainCheck>(
    `/v1/domains/${encodeURIComponent(domainId)}/checks`,
    context,
    { signal },
  );
}

export function getDomainReadiness(
  context: RequestContext,
  domainId: string,
  signal?: AbortSignal,
) {
  return requestJson<DomainReadiness>(
    `/v1/domains/${encodeURIComponent(domainId)}/readiness`,
    context,
    { signal },
  );
}

export function listApiKeys(context: RequestContext, signal?: AbortSignal) {
  return requestJson<ListResponse<ApiKey>>(
    listPath("/v1/api-keys", 50),
    context,
    { signal },
  );
}

export function getUsage(context: RequestContext, signal?: AbortSignal) {
  return requestJson<Usage>("/v1/usage", context, { signal });
}

export function listSubmissions(context: RequestContext, signal?: AbortSignal) {
  return requestJson<ListResponse<Submission>>(
    listPath("/v1/submissions", 20),
    context,
    { signal },
  );
}

export function getSendingConfig(context: RequestContext, signal?: AbortSignal) {
  return requestJson<SendingConfig>("/v1/sending/config", context, { signal });
}

export function listSubmissionAttempts(
  context: RequestContext,
  submissionId: string,
  signal?: AbortSignal,
) {
  return requestJson<ListResponse<SubmissionAttempt>>(
    `/v1/submissions/${encodeURIComponent(submissionId)}/attempts`,
    context,
    { signal },
  );
}

export function listMailboxes(context: RequestContext, signal?: AbortSignal) {
  return requestJson<ListResponse<Mailbox>>(
    listPath("/v1/mailboxes", 50),
    context,
    { signal },
  );
}

export function listMailboxMessages(
  context: RequestContext,
  mailboxId: string,
  signal?: AbortSignal,
) {
  return requestJson<ListResponse<Message>>(
    listPath(`/v1/mailboxes/${encodeURIComponent(mailboxId)}/messages`, 50),
    context,
    { signal },
  );
}

export function getMailboxMessage(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  signal?: AbortSignal,
) {
  return requestJson<Message>(
    `/v1/mailboxes/${encodeURIComponent(mailboxId)}/messages/${encodeURIComponent(messageId)}`,
    context,
    { signal },
  );
}

export function createDomain(context: RequestContext, domain: string) {
  return requestJson<Domain>("/v1/domains", context, {
    method: "POST",
    body: JSON.stringify({ domain }),
  });
}

export function createDomainChallenge(
  context: RequestContext,
  domainId: string,
) {
  return requestJson<VerificationChallenge>(
    `/v1/domains/${encodeURIComponent(domainId)}/verification`,
    context,
    { method: "POST" },
  );
}

export function confirmDomain(context: RequestContext, domainId: string) {
  return requestJson<{ status: "verified" | "pending_verification" }>(
    `/v1/domains/${encodeURIComponent(domainId)}/verification/check`,
    context,
    { method: "POST" },
  );
}

export function runDomainChecks(context: RequestContext, domainId: string) {
  return requestJson<DomainCheck>(
    `/v1/domains/${encodeURIComponent(domainId)}/checks`,
    context,
    { method: "POST" },
  );
}

export function disableDomain(context: RequestContext, domainId: string) {
  return requestJson<{ status: "disabled" }>(
    `/v1/domains/${encodeURIComponent(domainId)}`,
    context,
    { method: "DELETE" },
  );
}

export function createApiKey(
  context: RequestContext,
  request: {
    name: string;
    scopes: APIKeyScope[];
    expires_in?: string;
    expires_at?: string | null;
  },
) {
  return requestJson<CreatedApiKey>("/v1/api-keys", context, {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export function revokeApiKey(context: RequestContext, keyId: string) {
  return requestJson<{ status: "revoked" }>(
    `/v1/api-keys/${encodeURIComponent(keyId)}/revoke`,
    context,
    { method: "POST" },
  );
}

export function submitMessage(
  context: RequestContext,
  idempotencyKey: string,
  message: {
    from: string;
    to: string[];
    cc: string[];
    bcc: string[];
    subject: string;
    text: string;
  },
) {
  return requestJson<SubmissionAccepted>("/v1/submissions", context, {
    method: "POST",
    headers: { "Idempotency-Key": idempotencyKey },
    body: JSON.stringify(message),
  });
}
