"use client";

import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { RequestContext } from "@/lib/api/json-client";
import {
  appendMailboxMessage,
  assignMailboxMessageLabel,
  copyMailboxMessage,
  createMailboxAlias,
  createMailboxFolder,
  createMailboxLabel,
  deleteMailboxAlias,
  deleteMailboxFolder,
  expungeMailboxFolder,
  getMailboxDetails,
  getMailboxMessage,
  getMailboxMessageBody,
  getMailboxMessageContent,
  getMailboxMessageHeaders,
  getMailboxQuota,
  listMailboxAliases,
  listMailboxFolders,
  listMailboxLabels,
  listMailboxMessages,
  listMailboxPermissions,
  markMailboxMessageDeleted,
  moveMailboxMessage,
  provisionMailbox,
  removeMailboxMessageLabel,
  renameMailboxFolder,
  revokeMailboxPermission,
  setMailboxPermission,
  setMailboxQuota,
  updateMailboxMessageFlags,
  updateMailboxStatus,
} from "../services/mailbox-management-api";
import type { Domain, Mailbox, Message } from "../types/control-plane";
import type {
  MailboxAlias,
  MailboxDetails,
  MailboxFolder,
  MailboxLabel,
  MailboxPermission,
  MailboxQuota,
} from "../types/mailbox-management";

type DetailedMessage = Message & {
  folder_id: string;
  uid: number;
  flags: string[];
};

export function useMailboxManagement(
  context: RequestContext,
  mailboxes: Mailbox[],
  domains: Domain[],
  active: boolean,
  onMailboxCreated: () => Promise<void> = async () => {},
) {
  const availableMailboxes = mailboxes ?? [];
  const availableDomains = domains ?? [];
  const [requestedMailboxId, setMailboxId] = useState("");
  const mailboxId = availableMailboxes.some(
    (item) => item.id === requestedMailboxId,
  )
    ? requestedMailboxId
    : (availableMailboxes[0]?.id ?? "");
  const [details, setDetails] = useState<MailboxDetails | null>(null);
  const [quota, setQuota] = useState<MailboxQuota | null>(null);
  const [folders, setFolders] = useState<MailboxFolder[]>([]);
  const [aliases, setAliases] = useState<MailboxAlias[]>([]);
  const [permissions, setPermissions] = useState<MailboxPermission[]>([]);
  const [labels, setLabels] = useState<MailboxLabel[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [selectedMessage, setSelectedMessage] =
    useState<DetailedMessage | null>(null);
  const [messagePart, setMessagePart] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [busyAction, setBusyAction] = useState("");

  const [createDomainId, setCreateDomainId] = useState("");
  const [createLocalPart, setCreateLocalPart] = useState("");
  const [createDisplayName, setCreateDisplayName] = useState("");
  const [createKind, setCreateKind] = useState<"user" | "shared">("user");
  const [createQuota, setCreateQuota] = useState("1073741824");
  const [createOwnerId, setCreateOwnerId] = useState("");
  const [folderName, setFolderName] = useState("");
  const [folderParentId, setFolderParentId] = useState("");
  const [folderSpecialUse, setFolderSpecialUse] = useState("");
  const [aliasDomainId, setAliasDomainId] = useState("");
  const [aliasLocalPart, setAliasLocalPart] = useState("");
  const [permissionUserId, setPermissionUserId] = useState("");
  const [permissionLevel, setPermissionLevel] = useState<
    "read" | "read_write" | "manage"
  >("read");
  const [quotaInput, setQuotaInput] = useState("");
  const [labelName, setLabelName] = useState("");
  const [labelColor, setLabelColor] = useState("#475569");
  const [rawMessage, setRawMessage] = useState("");
  const [targetFolderId, setTargetFolderId] = useState("");

  const fallbackDomainId =
    availableDomains.find((item) => item.status === "verified")?.id ??
    availableDomains[0]?.id ??
    "";
  const effectiveCreateDomainId = availableDomains.some(
    (item) => item.id === createDomainId,
  )
    ? createDomainId
    : fallbackDomainId;
  const effectiveAliasDomainId = availableDomains.some(
    (item) => item.id === aliasDomainId,
  )
    ? aliasDomainId
    : fallbackDomainId;

  const load = useCallback(async () => {
    if (!mailboxId) {
      setDetails(null);
      setQuota(null);
      setFolders([]);
      setAliases([]);
      setPermissions([]);
      setLabels([]);
      setMessages([]);
      setSelectedMessage(null);
      return;
    }
    setLoading(true);
    setError("");
    const results = await Promise.allSettled([
      getMailboxDetails(context, mailboxId),
      getMailboxQuota(context, mailboxId),
      listMailboxFolders(context, mailboxId),
      listMailboxAliases(context, mailboxId),
      listMailboxPermissions(context, mailboxId),
      listMailboxLabels(context, mailboxId),
      listMailboxMessages(context, mailboxId),
    ]);
    const failures: string[] = [];
    if (results[0].status === "fulfilled") setDetails(results[0].value);
    else {
      setDetails(null);
      failures.push(
        results[0].reason instanceof Error
          ? results[0].reason.message
          : "Mailbox details could not be loaded.",
      );
    }
    if (results[1].status === "fulfilled") {
      setQuota(results[1].value);
      setQuotaInput(String(results[1].value.quota_bytes));
    } else {
      setQuota(null);
      failures.push(
        results[1].reason instanceof Error
          ? results[1].reason.message
          : "Mailbox quota could not be loaded.",
      );
    }
    if (results[2].status === "fulfilled") setFolders(results[2].value.items);
    else {
      setFolders([]);
      failures.push(
        results[2].reason instanceof Error
          ? results[2].reason.message
          : "Mailbox folders could not be loaded.",
      );
    }
    if (results[3].status === "fulfilled") setAliases(results[3].value.items);
    else {
      setAliases([]);
      failures.push(
        results[3].reason instanceof Error
          ? results[3].reason.message
          : "Mailbox aliases could not be loaded.",
      );
    }
    if (results[4].status === "fulfilled")
      setPermissions(results[4].value.items);
    else {
      setPermissions([]);
      failures.push(
        results[4].reason instanceof Error
          ? results[4].reason.message
          : "Mailbox access could not be loaded.",
      );
    }
    if (results[5].status === "fulfilled") setLabels(results[5].value.items);
    else {
      setLabels([]);
      failures.push(
        results[5].reason instanceof Error
          ? results[5].reason.message
          : "Mailbox labels could not be loaded.",
      );
    }
    if (results[6].status === "fulfilled") setMessages(results[6].value.items);
    else {
      setMessages([]);
      failures.push(
        results[6].reason instanceof Error
          ? results[6].reason.message
          : "Mailbox messages could not be loaded.",
      );
    }
    setError([...new Set(failures)].join(" "));
    setLoading(false);
  }, [context, mailboxId]);

  useEffect(() => {
    if (!active) return;
    const task = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(task);
  }, [active, load]);

  async function run(
    action: string,
    operation: () => Promise<string | void>,
    reload = true,
  ) {
    if (busyAction) return;
    setBusyAction(action);
    setError("");
    setMessage("");
    try {
      const success = await operation();
      if (success) setMessage(success);
      if (reload) await load();
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "The mailbox request failed.",
      );
    } finally {
      setBusyAction("");
    }
  }

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const quotaBytes = Number(createQuota);
    if (
      !createDomainId ||
      !Number.isSafeInteger(quotaBytes) ||
      quotaBytes < 1
    ) {
      setError("Choose a domain and a positive mailbox quota.");
      return;
    }
    await run(
      "mailbox:create",
      async () => {
        await provisionMailbox(context, {
          domain_id: effectiveCreateDomainId,
          local_part: createLocalPart.trim(),
          display_name: createDisplayName.trim(),
          kind: createKind,
          quota_bytes: quotaBytes,
          owner_user_id: createOwnerId.trim() || null,
        });
        setCreateLocalPart("");
        setCreateDisplayName("");
        setCreateOwnerId("");
        await onMailboxCreated();
        return "Mailbox provisioned with its default folders.";
      },
      false,
    );
  }

  async function setStatus(status: "active" | "suspended" | "deleting") {
    if (!mailboxId) return;
    await run(`mailbox:${status}`, async () => {
      await updateMailboxStatus(context, mailboxId, status);
      return `Mailbox status changed to ${status}.`;
    });
  }

  async function saveQuota(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = Number(quotaInput);
    if (!Number.isSafeInteger(value) || value < 1)
      return setError("Enter a positive whole-number quota in bytes.");
    await run("quota:update", async () => {
      await setMailboxQuota(context, mailboxId, value);
      return "Mailbox quota updated.";
    });
  }

  async function addFolder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run("folder:create", async () => {
      await createMailboxFolder(context, mailboxId, {
        name: folderName.trim(),
        parent_id: folderParentId || null,
        special_use: folderSpecialUse || null,
      });
      setFolderName("");
      setFolderParentId("");
      setFolderSpecialUse("");
      return "Folder created.";
    });
  }

  async function renameFolder(folder: MailboxFolder) {
    const name = window.prompt("New folder name", folder.name)?.trim();
    if (!name || name === folder.name) return;
    await run(`folder:rename:${folder.id}`, async () => {
      await renameMailboxFolder(context, mailboxId, folder.id, name);
      return "Folder renamed.";
    });
  }

  async function removeFolder(folder: MailboxFolder) {
    await run(`folder:delete:${folder.id}`, async () => {
      await deleteMailboxFolder(context, mailboxId, folder.id);
      return "Folder deleted.";
    });
  }

  async function expungeFolder(folderId: string) {
    await run(`folder:expunge:${folderId}`, async () => {
      const result = await expungeMailboxFolder(context, mailboxId, folderId);
      return `${result.expunged} deleted message(s) permanently expunged.`;
    });
  }

  async function addAlias(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run("alias:create", async () => {
      await createMailboxAlias(context, mailboxId, {
        domain_id: effectiveAliasDomainId,
        local_part: aliasLocalPart.trim(),
      });
      setAliasLocalPart("");
      return "Alias added.";
    });
  }

  async function removeAlias(alias: MailboxAlias) {
    await run(`alias:delete:${alias.id}`, async () => {
      await deleteMailboxAlias(context, mailboxId, alias.id);
      return "Alias removed.";
    });
  }

  async function grantPermission(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const userId = permissionUserId.trim();
    if (!/^[0-9a-f-]{36}$/i.test(userId))
      return setError("Enter a valid user UUID.");
    await run("permission:set", async () => {
      await setMailboxPermission(context, mailboxId, userId, permissionLevel);
      setPermissionUserId("");
      return "Mailbox access saved.";
    });
  }

  async function removePermission(permission: MailboxPermission) {
    await run(`permission:delete:${permission.user_id}`, async () => {
      await revokeMailboxPermission(context, mailboxId, permission.user_id);
      return "Mailbox access revoked.";
    });
  }

  async function addLabel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run("label:create", async () => {
      await createMailboxLabel(context, mailboxId, {
        name: labelName.trim(),
        kind: "label",
        color: labelColor,
      });
      setLabelName("");
      return "Label created.";
    });
  }

  async function toggleLabel(
    messageId: string,
    labelId: string,
    assigned: boolean,
  ) {
    await run(
      `message:label:${messageId}:${labelId}`,
      async () => {
        if (assigned)
          await removeMailboxMessageLabel(
            context,
            mailboxId,
            messageId,
            labelId,
          );
        else
          await assignMailboxMessageLabel(
            context,
            mailboxId,
            messageId,
            labelId,
          );
        return assigned
          ? "Label removed from message."
          : "Label assigned to message.";
      },
      false,
    );
  }

  async function loadMessage(messageId: string) {
    setError("");
    setMessagePart("");
    try {
      setSelectedMessage(
        await getMailboxMessage(context, mailboxId, messageId),
      );
    } catch (reason) {
      setSelectedMessage(null);
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not load message details.",
      );
    }
  }

  async function readMessagePart(part: "headers" | "body") {
    if (!selectedMessage) return;
    setBusyAction(`message:${part}`);
    setError("");
    try {
      setMessagePart(
        part === "headers"
          ? await getMailboxMessageHeaders(
              context,
              mailboxId,
              selectedMessage.id,
            )
          : await getMailboxMessageBody(context, mailboxId, selectedMessage.id),
      );
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : `Could not read message ${part}.`,
      );
    } finally {
      setBusyAction("");
    }
  }

  async function downloadMessage() {
    if (!selectedMessage) return;
    setBusyAction("message:download");
    setError("");
    try {
      const blob = await getMailboxMessageContent(
        context,
        mailboxId,
        selectedMessage.id,
      );
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `${selectedMessage.id}.eml`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not download message content.",
      );
    } finally {
      setBusyAction("");
    }
  }

  async function append(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await run("message:append", async () => {
      await appendMailboxMessage(
        context,
        mailboxId,
        rawMessage,
        window.crypto.randomUUID(),
      );
      setRawMessage("");
      return "Message appended to the selected mailbox.";
    });
  }

  async function saveFlags(flags: string[]) {
    if (!selectedMessage) return;
    await run("message:flags", async () => {
      await updateMailboxMessageFlags(
        context,
        mailboxId,
        selectedMessage.id,
        flags,
      );
      setSelectedMessage((current) =>
        current ? { ...current, flags } : current,
      );
      return "Message flags updated.";
    });
  }

  async function transferMessage(mode: "copy" | "move") {
    if (!selectedMessage || !targetFolderId)
      return setError("Choose a target folder.");
    await run(`message:${mode}`, async () => {
      if (mode === "copy")
        await copyMailboxMessage(
          context,
          mailboxId,
          selectedMessage.id,
          targetFolderId,
        );
      else
        await moveMailboxMessage(
          context,
          mailboxId,
          selectedMessage.id,
          targetFolderId,
        );
      setSelectedMessage(null);
      return mode === "copy" ? "Message copied." : "Message moved.";
    });
  }

  async function deleteMessage(item: Message) {
    await run(`message:delete:${item.id}`, async () => {
      await markMailboxMessageDeleted(context, mailboxId, item.id);
      setSelectedMessage(null);
      return "Message marked deleted. Use expunge on its folder to remove it permanently.";
    });
  }

  return {
    mailboxId,
    setMailboxId,
    details,
    quota,
    folders,
    aliases,
    permissions,
    labels,
    messages,
    selectedMessage,
    messagePart,
    loading,
    error,
    message,
    busyAction,
    createDomainId: effectiveCreateDomainId,
    setCreateDomainId,
    createLocalPart,
    setCreateLocalPart,
    createDisplayName,
    setCreateDisplayName,
    createKind,
    setCreateKind,
    createQuota,
    setCreateQuota,
    createOwnerId,
    setCreateOwnerId,
    folderName,
    setFolderName,
    folderParentId,
    setFolderParentId,
    folderSpecialUse,
    setFolderSpecialUse,
    aliasDomainId: effectiveAliasDomainId,
    setAliasDomainId,
    aliasLocalPart,
    setAliasLocalPart,
    permissionUserId,
    setPermissionUserId,
    permissionLevel,
    setPermissionLevel,
    quotaInput,
    setQuotaInput,
    labelName,
    setLabelName,
    labelColor,
    setLabelColor,
    rawMessage,
    setRawMessage,
    targetFolderId,
    setTargetFolderId,
    load,
    create,
    setStatus,
    saveQuota,
    addFolder,
    renameFolder,
    removeFolder,
    expungeFolder,
    addAlias,
    removeAlias,
    grantPermission,
    removePermission,
    addLabel,
    toggleLabel,
    loadMessage,
    readMessagePart,
    downloadMessage,
    append,
    saveFlags,
    transferMessage,
    deleteMessage,
  };
}
