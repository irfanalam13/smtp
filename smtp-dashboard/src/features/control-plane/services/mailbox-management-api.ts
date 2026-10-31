import { requestBlob, requestJson, requestText } from "@/lib/api/json-client";
import type { RequestContext } from "@/lib/api/json-client";
import type { ListResponse, Mailbox, Message } from "../types/control-plane";
import type {
  MailboxAlias,
  MailboxAppendResult,
  MailboxDetails,
  MailboxFolder,
  MailboxLabel,
  MailboxMessageResult,
  MailboxPermission,
  MailboxQuota,
} from "../types/mailbox-management";

function mailboxPath(mailboxId: string): string {
  return `/v1/mailboxes/${encodeURIComponent(mailboxId)}`;
}

function messagePath(mailboxId: string, messageId: string): string {
  return `${mailboxPath(mailboxId)}/messages/${encodeURIComponent(messageId)}`;
}

function jsonBody(method: string, body: unknown): RequestInit {
  return { method, body: JSON.stringify(body) };
}

export function getMailboxDetails(context: RequestContext, mailboxId: string) {
  return requestJson<MailboxDetails>(mailboxPath(mailboxId), context);
}

export function provisionMailbox(
  context: RequestContext,
  request: {
    domain_id: string;
    local_part: string;
    display_name: string;
    kind: "user" | "shared";
    quota_bytes: number;
    owner_user_id?: string | null;
  },
) {
  return requestJson<Mailbox>(
    "/v1/mailboxes",
    context,
    jsonBody("POST", request),
  );
}

export function updateMailboxStatus(
  context: RequestContext,
  mailboxId: string,
  status: "active" | "suspended" | "deleting",
) {
  return requestJson<{ id: string; status: string }>(
    mailboxPath(mailboxId),
    context,
    jsonBody("PATCH", { status }),
  );
}

export function listMailboxFolders(context: RequestContext, mailboxId: string) {
  return requestJson<ListResponse<MailboxFolder>>(
    `${mailboxPath(mailboxId)}/folders`,
    context,
  );
}

export function createMailboxFolder(
  context: RequestContext,
  mailboxId: string,
  request: {
    name: string;
    parent_id?: string | null;
    special_use?: string | null;
  },
) {
  return requestJson<MailboxFolder>(
    `${mailboxPath(mailboxId)}/folders`,
    context,
    jsonBody("POST", request),
  );
}

export function renameMailboxFolder(
  context: RequestContext,
  mailboxId: string,
  folderId: string,
  name: string,
) {
  return requestJson<MailboxFolder>(
    `${mailboxPath(mailboxId)}/folders/${encodeURIComponent(folderId)}`,
    context,
    jsonBody("PATCH", { name }),
  );
}

export function deleteMailboxFolder(
  context: RequestContext,
  mailboxId: string,
  folderId: string,
) {
  return requestJson<undefined>(
    `${mailboxPath(mailboxId)}/folders/${encodeURIComponent(folderId)}`,
    context,
    { method: "DELETE" },
  );
}

export function expungeMailboxFolder(
  context: RequestContext,
  mailboxId: string,
  folderId: string,
) {
  return requestJson<{ expunged: number }>(
    `${mailboxPath(mailboxId)}/folders/${encodeURIComponent(folderId)}/expunge`,
    context,
    { method: "POST" },
  );
}

export function listMailboxAliases(context: RequestContext, mailboxId: string) {
  return requestJson<ListResponse<MailboxAlias>>(
    `${mailboxPath(mailboxId)}/aliases`,
    context,
  );
}

export function createMailboxAlias(
  context: RequestContext,
  mailboxId: string,
  request: { domain_id: string; local_part: string },
) {
  return requestJson<{ id: string }>(
    `${mailboxPath(mailboxId)}/aliases`,
    context,
    jsonBody("POST", request),
  );
}

export function deleteMailboxAlias(
  context: RequestContext,
  mailboxId: string,
  aliasId: string,
) {
  return requestJson<undefined>(
    `${mailboxPath(mailboxId)}/aliases/${encodeURIComponent(aliasId)}`,
    context,
    { method: "DELETE" },
  );
}

export function listMailboxPermissions(
  context: RequestContext,
  mailboxId: string,
) {
  return requestJson<ListResponse<MailboxPermission>>(
    `${mailboxPath(mailboxId)}/permissions`,
    context,
  );
}

export function setMailboxPermission(
  context: RequestContext,
  mailboxId: string,
  userId: string,
  permission: "read" | "read_write" | "manage",
) {
  return requestJson<{ user_id: string; permission: string }>(
    `${mailboxPath(mailboxId)}/permissions/${encodeURIComponent(userId)}`,
    context,
    jsonBody("PUT", { permission }),
  );
}

export function revokeMailboxPermission(
  context: RequestContext,
  mailboxId: string,
  userId: string,
) {
  return requestJson<undefined>(
    `${mailboxPath(mailboxId)}/permissions/${encodeURIComponent(userId)}`,
    context,
    { method: "DELETE" },
  );
}

export function listMailboxMessages(
  context: RequestContext,
  mailboxId: string,
  limit = 50,
) {
  const query = new URLSearchParams({ limit: String(limit) });
  return requestJson<ListResponse<Message>>(
    `${mailboxPath(mailboxId)}/messages?${query}`,
    context,
  );
}

export function appendMailboxMessage(
  context: RequestContext,
  mailboxId: string,
  rawMessage: string,
  idempotencyKey: string,
  folderId?: string,
) {
  const query = folderId ? `?folder_id=${encodeURIComponent(folderId)}` : "";
  return requestJson<MailboxAppendResult>(
    `${mailboxPath(mailboxId)}/messages${query}`,
    context,
    {
      method: "POST",
      headers: {
        "Content-Type": "message/rfc822",
        "Idempotency-Key": idempotencyKey,
      },
      body: rawMessage,
    },
  );
}

export function getMailboxMessage(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
) {
  return requestJson<
    Message & { folder_id: string; uid: number; flags: string[] }
  >(messagePath(mailboxId, messageId), context);
}

export function markMailboxMessageDeleted(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
) {
  return requestJson<undefined>(messagePath(mailboxId, messageId), context, {
    method: "DELETE",
  });
}

export function getMailboxMessageContent(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
) {
  return requestBlob(`${messagePath(mailboxId, messageId)}/content`, context);
}

export function getMailboxMessageHeaders(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
) {
  return requestText(`${messagePath(mailboxId, messageId)}/headers`, context);
}

export function getMailboxMessageBody(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
) {
  return requestText(`${messagePath(mailboxId, messageId)}/body`, context);
}

export function updateMailboxMessageFlags(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  flags: string[],
) {
  return requestJson<{ id: string; flags: string[] }>(
    `${messagePath(mailboxId, messageId)}/flags`,
    context,
    jsonBody("PATCH", { flags }),
  );
}

export function copyMailboxMessage(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  targetFolderId: string,
) {
  return requestJson<MailboxMessageResult>(
    `${messagePath(mailboxId, messageId)}/copy`,
    context,
    jsonBody("POST", { target_folder_id: targetFolderId }),
  );
}

export function moveMailboxMessage(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  targetFolderId: string,
) {
  return requestJson<MailboxMessageResult>(
    `${messagePath(mailboxId, messageId)}/move`,
    context,
    jsonBody("POST", { target_folder_id: targetFolderId }),
  );
}

export function getMailboxQuota(context: RequestContext, mailboxId: string) {
  return requestJson<MailboxQuota>(`${mailboxPath(mailboxId)}/quota`, context);
}

export function setMailboxQuota(
  context: RequestContext,
  mailboxId: string,
  quotaBytes: number,
) {
  return requestJson<MailboxQuota>(
    `${mailboxPath(mailboxId)}/quota`,
    context,
    jsonBody("PUT", { quota_bytes: quotaBytes }),
  );
}

export function listMailboxLabels(context: RequestContext, mailboxId: string) {
  return requestJson<ListResponse<MailboxLabel>>(
    `${mailboxPath(mailboxId)}/labels`,
    context,
  );
}

export function createMailboxLabel(
  context: RequestContext,
  mailboxId: string,
  request: {
    name: string;
    kind: "label" | "category" | "tag" | "smart";
    color?: string;
    query_definition?: Record<string, unknown> | null;
  },
) {
  return requestJson<MailboxLabel>(
    `${mailboxPath(mailboxId)}/labels`,
    context,
    jsonBody("POST", request),
  );
}

export function assignMailboxMessageLabel(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  labelId: string,
) {
  return requestJson<undefined>(
    `${messagePath(mailboxId, messageId)}/labels/${encodeURIComponent(labelId)}`,
    context,
    { method: "PUT" },
  );
}

export function removeMailboxMessageLabel(
  context: RequestContext,
  mailboxId: string,
  messageId: string,
  labelId: string,
) {
  return requestJson<undefined>(
    `${messagePath(mailboxId, messageId)}/labels/${encodeURIComponent(labelId)}`,
    context,
    { method: "DELETE" },
  );
}
