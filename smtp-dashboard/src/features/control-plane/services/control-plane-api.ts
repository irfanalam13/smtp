import { requestJson } from "@/lib/api/json-client";
import type { RequestContext } from "@/lib/api/json-client";
import type {
  ApiKey,
  Domain,
  DomainCheck,
  Mailbox,
  Message,
  Principal,
  Submission,
  SubmissionAttempt,
  Usage,
  VerificationChallenge,
} from "../types/control-plane";

type ListResponse<T> = { items: T[] };

export function getReadiness(context: RequestContext) {
  return requestJson<{ status: string }>("/health/ready", context);
}

export function getPrincipal(context: RequestContext) {
  return requestJson<Principal>("/v1/me", context);
}

export function listDomains(context: RequestContext) {
  return requestJson<ListResponse<Domain>>("/v1/domains", context);
}

export function listApiKeys(context: RequestContext) {
  return requestJson<ListResponse<ApiKey>>("/v1/api-keys?limit=50", context);
}

export function getUsage(context: RequestContext) {
  return requestJson<Usage>("/v1/usage", context);
}

export function listSubmissions(context: RequestContext) {
  return requestJson<ListResponse<Submission>>(
    "/v1/submissions?limit=20",
    context,
  );
}

export function listSubmissionAttempts(
  context: RequestContext,
  submissionId: string,
) {
  return requestJson<ListResponse<SubmissionAttempt>>(
    `/v1/submissions/${encodeURIComponent(submissionId)}/attempts`,
    context,
  );
}

export function listMailboxes(context: RequestContext) {
  return requestJson<ListResponse<Mailbox>>("/v1/mailboxes?limit=50", context);
}

export function listMailboxMessages(
  context: RequestContext,
  mailboxId: string,
) {
  return requestJson<ListResponse<Message>>(
    `/v1/mailboxes/${encodeURIComponent(mailboxId)}/messages?limit=50`,
    context,
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
  return requestJson<{ status: string }>(
    `/v1/domains/${encodeURIComponent(domainId)}/verification/check`,
    context,
    { method: "POST" },
  );
}

export function checkDomain(context: RequestContext, domainId: string) {
  return requestJson<{ results: DomainCheck[] }>(
    `/v1/domains/${encodeURIComponent(domainId)}/checks`,
    context,
    { method: "POST" },
  );
}

export function createApiKey(context: RequestContext, name: string) {
  return requestJson<{ secret: string }>("/v1/api-keys", context, {
    method: "POST",
    body: JSON.stringify({
      name,
      scopes: ["domains:read", "delivery:read", "delivery:write"],
    }),
  });
}

export function revokeApiKey(context: RequestContext, keyId: string) {
  return requestJson<{ status: string }>(
    `/v1/api-keys/${encodeURIComponent(keyId)}/revoke`,
    context,
    { method: "POST" },
  );
}

export function submitMessage(
  context: RequestContext,
  idempotencyKey: string,
  message: { from: string; to: string[]; subject: string; text: string },
) {
  return requestJson<{ submission: Submission }>("/v1/submissions", context, {
    method: "POST",
    headers: { "Idempotency-Key": idempotencyKey },
    body: JSON.stringify(message),
  });
}
