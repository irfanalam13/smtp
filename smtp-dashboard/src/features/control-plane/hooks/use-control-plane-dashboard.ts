"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import { usePathname, useRouter } from "next/navigation";
import { ApiError } from "@/lib/api/json-client";
import {
  confirmDomain as confirmDomainRequest,
  createApiKey,
  createDomain,
  createDomainChallenge,
  disableDomain as disableDomainRequest,
  getDomainChecks,
  getDomainReadiness,
  getMailboxMessage,
  getPrincipal,
  getReadiness,
  getSendingConfig,
  getTenant,
  getUsage,
  listApiKeys,
  listDomains,
  listMailboxMessages,
  listMailboxes,
  listSubmissions,
  listSubmissionAttempts,
  revokeApiKey,
  runDomainChecks as runDomainChecksRequest,
  submitMessage,
} from "../services/control-plane-api";
import type {
  APIKeyScope,
  ApiKey,
  Domain,
  DomainCheck,
  DomainReadiness,
  DashboardView,
  LoadErrors,
  Mailbox,
  Message,
  Principal,
  Submission,
  SubmissionAttempt,
  SendingConfig,
  Tenant,
  Usage,
  VerificationChallenge,
} from "../types/control-plane";
import { validateDomainName, validateSubmission } from "../utils/validators";
import type { ComposeField } from "../utils/validators";
import {
  dashboardPathForView,
  dashboardViewForPath,
} from "../routing/dashboard-routes";

type LoadResult<T> = { ok: true; value: T } | { ok: false; error: unknown };

async function capture<T>(request: Promise<T>): Promise<LoadResult<T>> {
  try {
    return { ok: true, value: await request };
  } catch (error) {
    return { ok: false, error };
  }
}

function messageOf(reason: unknown): string {
  if (reason instanceof ApiError) {
    const retry = reason.retryAfterSeconds;
    const retryMessage =
      reason.category === "rate_limited" && retry !== null
        ? ` Retry after ${retry} seconds.`
        : "";
    const requestId = reason.requestId
      ? ` Request ID: ${reason.requestId}.`
      : "";
    return `${reason.message}${retryMessage}${requestId}`;
  }
  if (
    typeof reason === "object" &&
    reason !== null &&
    "name" in reason &&
    reason.name === "AbortError"
  ) {
    return "The request was cancelled.";
  }
  return reason instanceof Error ? reason.message : "The request failed.";
}

function emptyDashboardData() {
  return {
    domains: [] as Domain[],
    keys: [] as ApiKey[],
    mailboxes: [] as Mailbox[],
    messages: [] as Message[],
    submissions: [] as Submission[],
    usage: null as Usage | null,
  };
}

const initialKeyScopes: APIKeyScope[] = ["delivery:read", "delivery:write"];

export function useControlPlaneDashboard() {
  const pathname = usePathname() ?? "/";
  const router = useRouter();
  const view = dashboardViewForPath(pathname) ?? "home";
  const [apiToken, setApiToken] = useState("");
  const [tokenInput, setTokenInput] = useState("");
  const [tenantID, setTenantID] = useState("");
  const [tenantInput, setTenantInput] = useState("");
  const [principal, setPrincipal] = useState<Principal | null>(null);
  const [tenant, setTenant] = useState<Tenant | null>(null);
  const [principalError, setPrincipalError] = useState("");
  const [connected, setConnected] = useState(false);
  const [readiness, setReadiness] = useState<"ready" | "not_ready" | null>(
    null,
  );
  const [connectionError, setConnectionError] = useState("");
  const [loading, setLoading] = useState(true);
  const [domains, setDomains] = useState<Domain[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [mailboxes, setMailboxes] = useState<Mailbox[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [submissions, setSubmissions] = useState<Submission[]>([]);
  const [sendingConfig, setSendingConfig] = useState<SendingConfig>({
    relay_ready: false,
    from: "",
  });
  const [activitySubmissionId, setActivitySubmissionId] = useState("");
  const [submissionAttempts, setSubmissionAttempts] = useState<
    SubmissionAttempt[]
  >([]);
  const [activityLoading, setActivityLoading] = useState(false);
  const [activityError, setActivityError] = useState("");
  const [usage, setUsage] = useState<Usage | null>(null);
  const [loadErrors, setLoadErrors] = useState<LoadErrors>({});
  const [actionError, setActionError] = useState("");
  const [actionMessage, setActionMessage] = useState("");
  const [busyAction, setBusyAction] = useState("");
  const [domainChecks, setDomainChecks] = useState<Record<string, DomainCheck>>(
    {},
  );
  const [domainReadiness, setDomainReadiness] = useState<
    Record<string, DomainReadiness>
  >({});
  const [domainDetailsLoading, setDomainDetailsLoading] = useState<
    Record<string, boolean>
  >({});
  const [domainDetailsErrors, setDomainDetailsErrors] = useState<
    Record<string, string>
  >({});
  const [challenges, setChallenges] = useState<
    Record<string, VerificationChallenge>
  >({});
  const [domainInput, setDomainInput] = useState("");
  const [domainInputError, setDomainInputError] = useState("");
  const [keyName, setKeyName] = useState("");
  const [keyNameError, setKeyNameError] = useState("");
  const [keyScopes, setKeyScopes] = useState<APIKeyScope[]>(initialKeyScopes);
  const [keyExpiresAt, setKeyExpiresAt] = useState("");
  const [keyExpiryPreset, setKeyExpiryPreset] = useState("1_month");
  const [oneTimeSecret, setOneTimeSecret] = useState("");
  const [secretWarning, setSecretWarning] = useState("");
  const [secretCopied, setSecretCopied] = useState(false);
  const [selectedMailbox, setSelectedMailbox] = useState("");
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [messageError, setMessageError] = useState("");
  const [selectedMessage, setSelectedMessage] = useState<Message | null>(null);
  const [selectedMessageId, setSelectedMessageId] = useState("");
  const [messageDetailLoading, setMessageDetailLoading] = useState(false);
  const [sender, setSender] = useState("");
  const [recipients, setRecipients] = useState("");
  const [cc, setCc] = useState("");
  const [bcc, setBcc] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [composeErrors, setComposeErrors] = useState<
    Partial<Record<ComposeField, string>>
  >({});
  const [sending, setSending] = useState(false);
  const [submissionIdempotencyKey, setSubmissionIdempotencyKey] = useState("");

  const requestContext = useMemo(
    () => ({ token: apiToken, tenantId: tenantID }),
    [apiToken, tenantID],
  );
  const verifiedDomains = useMemo(
    () => domains.filter((item) => item.status === "verified").length,
    [domains],
  );
  const selectedMailboxRef = useRef("");
  const loadGeneration = useRef(0);
  const activeLoadController = useRef<AbortController | null>(null);
  const busyActionRef = useRef("");
  const activityRequest = useRef(0);
  const activityController = useRef<AbortController | null>(null);
  const mailboxRequest = useRef(0);
  const mailboxController = useRef<AbortController | null>(null);
  const messageRequest = useRef(0);
  const messageController = useRef<AbortController | null>(null);
  const domainDetailControllers = useRef(new Map<string, AbortController>());
  const sendingRef = useRef(false);

  function setView(nextView: DashboardView) {
    if (nextView !== "api-keys") {
      setOneTimeSecret("");
      setSecretWarning("");
      setSecretCopied(false);
    }
    router.push(dashboardPathForView(nextView));
  }

  const load = useCallback(async () => {
    activeLoadController.current?.abort();
    const controller = new AbortController();
    activeLoadController.current = controller;
    const generation = ++loadGeneration.current;
    const context = { token: apiToken, tenantId: tenantID };
    const isCurrent = () =>
      generation === loadGeneration.current && !controller.signal.aborted;
    setLoading(true);

    try {
      const [
        readinessResult,
        principalResult,
        tenantResult,
        domainsResult,
        keysResult,
        usageResult,
        submissionsResult,
        mailboxesResult,
        sendingConfigResult,
      ] = await Promise.all([
        capture(getReadiness(context, controller.signal)),
        capture(getPrincipal(context, controller.signal)),
        capture(getTenant(context, controller.signal)),
        capture(listDomains(context, controller.signal)),
        capture(listApiKeys(context, controller.signal)),
        capture(getUsage(context, controller.signal)),
        capture(listSubmissions(context, controller.signal)),
        capture(listMailboxes(context, controller.signal)),
        capture(getSendingConfig(context, controller.signal)),
      ]);
      if (!isCurrent()) return;

      const errors: LoadErrors = {};
      if (readinessResult.ok) {
        setReadiness(readinessResult.value.status);
        setConnected(readinessResult.value.status === "ready");
        setConnectionError(
          readinessResult.value.status === "ready"
            ? ""
            : "The API is not ready to serve requests.",
        );
        if (readinessResult.value.status !== "ready") {
          errors.readiness = "The API is not ready to serve requests.";
        }
      } else {
        setReadiness(null);
        setConnected(false);
        setConnectionError(messageOf(readinessResult.error));
        errors.readiness = messageOf(readinessResult.error);
      }

      if (principalResult.ok) {
        setPrincipal(principalResult.value);
        setPrincipalError("");
      } else {
        setPrincipal(null);
        setPrincipalError(messageOf(principalResult.error));
        errors.principal = messageOf(principalResult.error);
      }
      if (tenantResult.ok) {
        setTenant(tenantResult.value);
      } else {
        setTenant(principalResult.ok ? principalResult.value.tenant : null);
        errors.tenant = messageOf(tenantResult.error);
      }

      if (domainsResult.ok) {
        setDomains(domainsResult.value.items);
      } else {
        setDomains([]);
        errors.domains = messageOf(domainsResult.error);
      }
      if (keysResult.ok) {
        setKeys(keysResult.value.items);
      } else {
        setKeys([]);
        errors.keys = messageOf(keysResult.error);
      }
      if (usageResult.ok) {
        setUsage(usageResult.value);
      } else {
        setUsage(null);
        errors.usage = messageOf(usageResult.error);
      }
      if (submissionsResult.ok) {
        setSubmissions(submissionsResult.value.items);
      } else {
        setSubmissions([]);
        errors.submissions = messageOf(submissionsResult.error);
      }
      if (sendingConfigResult.ok) {
        setSendingConfig(sendingConfigResult.value);
        setSender((current) => current || sendingConfigResult.value.from);
      } else {
        setSendingConfig({
          relay_ready: false,
          from: "",
          error: messageOf(sendingConfigResult.error),
        });
      }

      if (!mailboxesResult.ok) {
        setMailboxes([]);
        setMessages([]);
        setSelectedMailbox("");
        selectedMailboxRef.current = "";
        setSelectedMessage(null);
        setSelectedMessageId("");
        errors.mailboxes = messageOf(mailboxesResult.error);
      } else {
        const nextMailboxes = mailboxesResult.value.items;
        const nextMailboxId =
          nextMailboxes.find((item) => item.id === selectedMailboxRef.current)
            ?.id ??
          nextMailboxes[0]?.id ??
          "";
        setMailboxes(nextMailboxes);
        setSelectedMailbox(nextMailboxId);
        selectedMailboxRef.current = nextMailboxId;
        setSelectedMessage(null);
        setSelectedMessageId("");

        if (nextMailboxId) {
          setMessagesLoading(true);
          const messagesResult = await capture(
            listMailboxMessages(context, nextMailboxId, controller.signal),
          );
          if (!isCurrent()) return;
          if (messagesResult.ok) {
            setMessages(messagesResult.value.items);
          } else {
            setMessages([]);
            errors.mailboxes = messageOf(messagesResult.error);
          }
          setMessagesLoading(false);
        } else {
          setMessages([]);
          setMessagesLoading(false);
        }
      }
      if (isCurrent()) {
        setLoadErrors(errors);
      }
    } finally {
      if (isCurrent()) {
        setLoading(false);
        activeLoadController.current = null;
      }
    }
  }, [apiToken, tenantID]);

  useEffect(() => {
    let active = true;
    void Promise.resolve()
      .then(() => (active ? load() : undefined))
      .catch((error: unknown) => {
        if (!active) return;
        setConnectionError(messageOf(error));
        setLoading(false);
      });
    return () => {
      active = false;
      loadGeneration.current += 1;
      activeLoadController.current?.abort();
    };
  }, [load]);

  useEffect(
    () => () => {
      activeLoadController.current?.abort();
      activityController.current?.abort();
      mailboxController.current?.abort();
      messageController.current?.abort();
      domainDetailControllers.current.forEach((controller) =>
        controller.abort(),
      );
      domainDetailControllers.current.clear();
    },
    [],
  );

  function resetWorkspaceData() {
    const empty = emptyDashboardData();
    setPrincipal(null);
    setTenant(null);
    setReadiness(null);
    setDomains(empty.domains);
    setKeys(empty.keys);
    setMailboxes(empty.mailboxes);
    setMessages(empty.messages);
    setSubmissions(empty.submissions);
    setUsage(empty.usage);
    setLoadErrors({});
    setDomainChecks({});
    setDomainReadiness({});
    setChallenges({});
    setSelectedMailbox("");
    selectedMailboxRef.current = "";
    setSelectedMessage(null);
    setSelectedMessageId("");
    setMessageDetailLoading(false);
    setMessageError("");
    setOneTimeSecret("");
    setSecretWarning("");
    setSecretCopied(false);
    setActionError("");
    setActionMessage("");
    setPrincipalError("");
    setConnected(false);
    setConnectionError("");
  }

  function applyToken(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    activityController.current?.abort();
    mailboxController.current?.abort();
    messageController.current?.abort();
    resetWorkspaceData();
    setActivitySubmissionId("");
    setSubmissionAttempts([]);
    setActivityLoading(false);
    setActivityError("");
    setTokenInput(tokenInput.trim());
    setTenantInput(tenantInput.trim());
    setApiToken(tokenInput.trim());
    setTenantID(tenantInput.trim());
  }

  function clearToken() {
    activeLoadController.current?.abort();
    activityController.current?.abort();
    mailboxController.current?.abort();
    messageController.current?.abort();
    setTokenInput("");
    setApiToken("");
    setTenantInput("");
    setTenantID("");
    setActivitySubmissionId("");
    setSubmissionAttempts([]);
    setActivityLoading(false);
    setActivityError("");
    resetWorkspaceData();
  }

  async function runAction(action: string, operation: () => Promise<void>) {
    if (busyActionRef.current) return;
    busyActionRef.current = action;
    setBusyAction(action);
    setActionError("");
    setActionMessage("");
    try {
      await operation();
    } catch (error) {
      setActionError(messageOf(error));
    } finally {
      busyActionRef.current = "";
      setBusyAction("");
    }
  }

  async function addDomain(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validationError = validateDomainName(domainInput);
    setDomainInputError(validationError ?? "");
    if (validationError) return;

    await runAction("domain:add", async () => {
      await createDomain(requestContext, domainInput);
      setDomainInput("");
      setActionMessage("Domain registration was accepted by the API.");
      await load();
    });
  }

  async function loadDomainDetails(domainId: string) {
    domainDetailControllers.current.get(domainId)?.abort();
    const controller = new AbortController();
    domainDetailControllers.current.set(domainId, controller);
    setDomainDetailsLoading((current) => ({ ...current, [domainId]: true }));
    setDomainDetailsErrors((current) => ({ ...current, [domainId]: "" }));

    const [checksResult, readinessResult] = await Promise.all([
      capture(getDomainChecks(requestContext, domainId, controller.signal)),
      capture(getDomainReadiness(requestContext, domainId, controller.signal)),
    ]);
    if (domainDetailControllers.current.get(domainId) !== controller) return;

    const failures: string[] = [];
    if (checksResult.ok) {
      setDomainChecks((current) => ({
        ...current,
        [domainId]: checksResult.value,
      }));
    } else {
      failures.push(messageOf(checksResult.error));
    }
    if (readinessResult.ok) {
      setDomainReadiness((current) => ({
        ...current,
        [domainId]: readinessResult.value,
      }));
    } else {
      failures.push(messageOf(readinessResult.error));
    }
    setDomainDetailsErrors((current) => ({
      ...current,
      [domainId]: failures.join(" "),
    }));
    setDomainDetailsLoading((current) => ({ ...current, [domainId]: false }));
    domainDetailControllers.current.delete(domainId);
  }

  async function createChallenge(domainId: string) {
    await runAction(`domain:challenge:${domainId}`, async () => {
      const challenge = await createDomainChallenge(requestContext, domainId);
      setChallenges((current) => ({ ...current, [domainId]: challenge }));
      setActionMessage("The DNS verification challenge was created.");
    });
  }

  async function confirmOwnership(domainId: string) {
    await runAction(`domain:verify:${domainId}`, async () => {
      const result = await confirmDomainRequest(requestContext, domainId);
      setActionMessage(
        result.status === "verified"
          ? "The API verified domain ownership."
          : "The verification record was not observed yet. Check DNS and retry.",
      );
      await load();
    });
  }

  async function runDomainChecks(domainId: string) {
    await runAction(`domain:checks:${domainId}`, async () => {
      const result = await runDomainChecksRequest(requestContext, domainId);
      setDomainChecks((current) => ({ ...current, [domainId]: result }));
      setActionMessage("DNS diagnostics completed.");
      await loadDomainDetails(domainId);
    });
  }

  async function disableDomain(domain: Domain) {
    await runAction(`domain:disable:${domain.id}`, async () => {
      await disableDomainRequest(requestContext, domain.id);
      setChallenges((current) => {
        const next = { ...current };
        delete next[domain.id];
        return next;
      });
      setDomainChecks((current) => {
        const next = { ...current };
        delete next[domain.id];
        return next;
      });
      setDomainReadiness((current) => {
        const next = { ...current };
        delete next[domain.id];
        return next;
      });
      setActionMessage(
        "The sending domain was disabled. New submissions from it will be rejected.",
      );
      await load();
    });
  }

  async function addApiKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalizedName = keyName.trim();
    const nameError =
      normalizedName.length === 0
        ? "Enter a name for this API key."
        : new TextEncoder().encode(keyName).length > 120
          ? "API-key names must be 120 bytes or fewer."
          : "";
    setKeyNameError(nameError);
    if (nameError) return;
    if (keyScopes.length < 1 || keyScopes.length > 10) {
      setActionError("Select between 1 and 10 API-key scopes.");
      return;
    }

    const expiry = keyExpiresAt ? new Date(keyExpiresAt) : null;
    if (
      keyExpiryPreset === "custom" &&
      (!expiry || Number.isNaN(expiry.getTime()) || expiry <= new Date())
    ) {
      setActionError("Choose an API-key expiry in the future.");
      return;
    }

    await runAction("key:create", async () => {
      const created = await createApiKey(requestContext, {
        name: keyName,
        scopes: keyScopes,
        ...(keyExpiryPreset === "custom"
          ? { expires_at: expiry?.toISOString() ?? null }
          : keyExpiryPreset
            ? { expires_in: keyExpiryPreset }
            : {}),
      });
      setOneTimeSecret(created.secret);
      setSecretWarning(created.warning);
      setSecretCopied(false);
      setKeyName("");
      setKeyScopes(initialKeyScopes);
      setKeyExpiresAt("");
      setKeyExpiryPreset("1_month");
      setView("api-keys");
      await load();
    });
  }

  function toggleKeyScope(scope: APIKeyScope, selected: boolean) {
    setKeyScopes((current) => {
      if (selected) {
        return current.includes(scope) ? current : [...current, scope];
      }
      return current.filter((value) => value !== scope);
    });
  }

  async function copyApiKey() {
    if (!oneTimeSecret) return;
    if (!navigator.clipboard) {
      setActionError("Clipboard access is unavailable in this browser.");
      return;
    }
    try {
      await navigator.clipboard.writeText(oneTimeSecret);
      setSecretCopied(true);
      setActionError("");
    } catch (error) {
      setActionError(`Could not copy the API key: ${messageOf(error)}`);
    }
  }

  function dismissApiKeySecret() {
    setOneTimeSecret("");
    setSecretWarning("");
    setSecretCopied(false);
  }

  async function removeApiKey(keyId: string) {
    await runAction(`key:revoke:${keyId}`, async () => {
      await revokeApiKey(requestContext, keyId);
      setActionMessage("The API key was revoked.");
      await load();
    });
  }

  async function selectMailbox(mailboxId: string) {
    mailboxController.current?.abort();
    messageController.current?.abort();
    const controller = new AbortController();
    mailboxController.current = controller;
    const requestId = ++mailboxRequest.current;
    selectedMailboxRef.current = mailboxId;
    setSelectedMailbox(mailboxId);
    setSelectedMessage(null);
    setSelectedMessageId("");
    setMessageDetailLoading(false);
    setMessageError("");
    setMessages([]);
    if (!mailboxId) {
      setMessagesLoading(false);
      return;
    }

    setMessagesLoading(true);
    const result = await capture(
      listMailboxMessages(requestContext, mailboxId, controller.signal),
    );
    if (requestId !== mailboxRequest.current || controller.signal.aborted) {
      return;
    }
    if (result.ok) {
      setMessages(result.value.items);
      setLoadErrors((current) => ({ ...current, mailboxes: undefined }));
    } else {
      setLoadErrors((current) => ({
        ...current,
        mailboxes: messageOf(result.error),
      }));
    }
    setMessagesLoading(false);
  }

  async function selectMessage(messageId: string) {
    messageController.current?.abort();
    const controller = new AbortController();
    messageController.current = controller;
    const requestId = ++messageRequest.current;
    setSelectedMessageId(messageId);
    setSelectedMessage(null);
    setMessageError("");
    if (!messageId || !selectedMailboxRef.current) {
      setMessageDetailLoading(false);
      return;
    }

    setMessageDetailLoading(true);
    const result = await capture(
      getMailboxMessage(
        requestContext,
        selectedMailboxRef.current,
        messageId,
        controller.signal,
      ),
    );
    if (requestId !== messageRequest.current || controller.signal.aborted) {
      return;
    }
    if (result.ok) {
      setSelectedMessage(result.value);
    } else {
      setMessageError(messageOf(result.error));
    }
    setMessageDetailLoading(false);
  }

  async function loadSubmissionAttempts(submissionId: string) {
    activityController.current?.abort();
    const controller = new AbortController();
    activityController.current = controller;
    const requestId = ++activityRequest.current;
    setActivitySubmissionId(submissionId);
    setActivityError("");
    setSubmissionAttempts([]);
    if (!submissionId) {
      setActivityLoading(false);
      return;
    }

    setActivityLoading(true);
    const result = await capture(
      listSubmissionAttempts(requestContext, submissionId, controller.signal),
    );
    if (requestId !== activityRequest.current || controller.signal.aborted) {
      return;
    }
    if (result.ok) {
      setSubmissionAttempts(result.value.items);
    } else {
      setActivityError(messageOf(result.error));
    }
    setActivityLoading(false);
  }

  function updateComposeField(
    field: ComposeField,
    setter: (value: string) => void,
    value: string,
  ) {
    setter(value);
    setComposeErrors((current) => ({ ...current, [field]: undefined }));
    setSubmissionIdempotencyKey("");
  }

  async function sendMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (sendingRef.current) return;
    setActionError("");
    setActionMessage("");

    const validation = validateSubmission({
      from: sender,
      to: recipients,
      cc,
      bcc,
      subject,
      body,
    });
    setComposeErrors(validation.errors);
    if (Object.keys(validation.errors).length > 0) return;

    sendingRef.current = true;
    setSending(true);
    try {
      const idempotencyKey =
        submissionIdempotencyKey || window.crypto.randomUUID();
      if (!submissionIdempotencyKey) {
        setSubmissionIdempotencyKey(idempotencyKey);
      }
      const accepted = await submitMessage(requestContext, idempotencyKey, {
        from: sender,
        to: validation.to,
        cc: validation.cc,
        bcc: validation.bcc,
        subject,
        text: body,
      });
      setSubmissions((current) => [
        accepted.submission,
        ...current.filter((item) => item.id !== accepted.submission.id),
      ]);
      setActionMessage(
        accepted.created
          ? accepted.message
          : `This idempotent retry refers to the earlier submission. ${accepted.message}`,
      );
      setSubmissionIdempotencyKey("");
      setSender("");
      setRecipients("");
      setCc("");
      setBcc("");
      setSubject("");
      setBody("");
      setComposeErrors({});
      await load();
    } catch (error) {
      setActionError(messageOf(error));
    } finally {
      sendingRef.current = false;
      setSending(false);
    }
  }

  return {
    view,
    setView,
    requestContext,
    apiToken,
    tokenInput,
    setTokenInput,
    tenantInput,
    setTenantInput,
    principal,
    tenant,
    principalError,
    connected,
    readiness,
    connectionError,
    loading,
    domains,
    keys,
    mailboxes,
    messages,
    messagesLoading,
    messageError,
    selectedMessage,
    selectedMessageId,
    messageDetailLoading,
    submissions,
    sendingConfig,
    activitySubmissionId,
    submissionAttempts,
    activityLoading,
    activityError,
    usage,
    loadErrors,
    actionError,
    setActionError,
    actionMessage,
    setActionMessage,
    busyAction,
    domainChecks,
    domainReadiness,
    domainDetailsLoading,
    domainDetailsErrors,
    challenges,
    domainInput,
    domainInputError,
    setDomainInput: (value: string) => {
      setDomainInput(value);
      setDomainInputError("");
    },
    keyName,
    keyNameError,
    setKeyName: (value: string) => {
      setKeyName(value);
      setKeyNameError("");
    },
    keyScopes,
    keyExpiresAt,
    setKeyExpiresAt,
    keyExpiryPreset,
    setKeyExpiryPreset,
    toggleKeyScope,
    oneTimeSecret,
    secretWarning,
    secretCopied,
    copyApiKey,
    dismissApiKeySecret,
    selectedMailbox,
    sender,
    setSender: (value: string) => updateComposeField("from", setSender, value),
    recipients,
    setRecipients: (value: string) =>
      updateComposeField("to", setRecipients, value),
    cc,
    setCc: (value: string) => updateComposeField("cc", setCc, value),
    bcc,
    setBcc: (value: string) => updateComposeField("bcc", setBcc, value),
    subject,
    setSubject: (value: string) =>
      updateComposeField("subject", setSubject, value),
    body,
    setBody: (value: string) => updateComposeField("body", setBody, value),
    composeErrors,
    updateComposeField,
    sending,
    submissionIdempotencyKey,
    verifiedDomains,
    load,
    applyToken,
    clearToken,
    addDomain,
    loadDomainDetails,
    createChallenge,
    confirmOwnership,
    runDomainChecks,
    disableDomain,
    addApiKey,
    removeApiKey,
    selectMailbox,
    selectMessage,
    loadSubmissionAttempts,
    sendMessage,
  };
}
