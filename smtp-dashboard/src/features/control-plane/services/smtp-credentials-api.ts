import { requestJson } from "@/lib/api/json-client";
import type { RequestContext } from "@/lib/api/json-client";
import type { ListResponse } from "../types/control-plane";
import type {
  CreatedSMTPCredential,
  SMTPCredential,
} from "../types/smtp-credentials";

export function listSMTPCredentials(context: RequestContext) {
  return requestJson<ListResponse<SMTPCredential>>(
    "/v1/smtp-credentials?limit=100",
    context,
  );
}

export function createSMTPCredential(
  context: RequestContext,
  username: string,
  expiresAt: string,
) {
  return requestJson<CreatedSMTPCredential>("/v1/smtp-credentials", context, {
    method: "POST",
    body: JSON.stringify({ username, expires_at: expiresAt }),
  });
}

export function rotateSMTPCredential(
  context: RequestContext,
  credentialId: string,
  expiresAt: string,
) {
  return requestJson<CreatedSMTPCredential>(
    `/v1/smtp-credentials/${encodeURIComponent(credentialId)}/rotate`,
    context,
    { method: "POST", body: JSON.stringify({ expires_at: expiresAt }) },
  );
}

export function revokeSMTPCredential(
  context: RequestContext,
  credentialId: string,
) {
  return requestJson<{ status: "revoked" }>(
    `/v1/smtp-credentials/${encodeURIComponent(credentialId)}/revoke`,
    context,
    { method: "POST" },
  );
}
