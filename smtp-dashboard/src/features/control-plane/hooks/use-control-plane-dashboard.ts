"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FormEvent } from "react";
import {
  checkDomain as checkDomainRequest,
  confirmDomain as confirmDomainRequest,
  createApiKey,
  createDomain,
  createDomainChallenge,
  getPrincipal,
  getReadiness,
  getUsage,
  listApiKeys,
  listDomains,
  listMailboxMessages,
  listMailboxes,
  listSubmissions,
  listSubmissionAttempts,
  revokeApiKey,
  submitMessage,
} from "../services/control-plane-api";
import type {
  ApiKey,
  Domain,
  DomainCheck,
  LoadErrors,
  Mailbox,
  Message,
  Principal,
  Submission,
  SubmissionAttempt,
  Usage,
  VerificationChallenge,
  DashboardView,
} from "../types/control-plane";

function messageOf(reason: unknown): string {
  return reason instanceof Error ? reason.message : "The request failed";
}

export function useControlPlaneDashboard() {
  const activityRequest = useRef(0);
  const [view, setView] = useState<DashboardView>("home");
  const [apiToken, setApiToken] = useState("");
  const [tokenInput, setTokenInput] = useState("");
  const [tenantID, setTenantID] = useState("");
  const [tenantInput, setTenantInput] = useState("");
  const [principal, setPrincipal] = useState<Principal | null>(null);
  const [principalError, setPrincipalError] = useState("");
  const [connected, setConnected] = useState(false);
  const [connectionError, setConnectionError] = useState("");
  const [loading, setLoading] = useState(true);
  const [domains, setDomains] = useState<Domain[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [mailboxes, setMailboxes] = useState<Mailbox[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [submissions, setSubmissions] = useState<Submission[]>([]);
  const [activitySubmissionId, setActivitySubmissionId] = useState("");
  const [submissionAttempts, setSubmissionAttempts] = useState<
    SubmissionAttempt[]
  >([]);
  const [activityLoading, setActivityLoading] = useState(false);
  const [activityError, setActivityError] = useState("");
  const [usage, setUsage] = useState<Usage | null>(null);
  const [loadErrors, setLoadErrors] = useState<LoadErrors>({});
  const [actionError, setActionError] = useState("");
  const [domainChecks, setDomainChecks] = useState<
    Record<string, DomainCheck[]>
  >({});
  const [challenges, setChallenges] = useState<
    Record<string, VerificationChallenge>
  >({});
  const [domainInput, setDomainInput] = useState("");
  const [keyName, setKeyName] = useState("");
  const [oneTimeSecret, setOneTimeSecret] = useState("");
  const [secretCopied, setSecretCopied] = useState(false);
  const [selectedMailbox, setSelectedMailbox] = useState("");
  const [sender, setSender] = useState("");
  const [recipients, setRecipients] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [sending, setSending] = useState(false);
  const [submissionIdempotencyKey, setSubmissionIdempotencyKey] = useState("");

  const verifiedDomains = useMemo(
    () => domains.filter((item) => item.status === "verified").length,
    [domains],
  );
  const requestContext = useMemo(
    () => ({ token: apiToken, tenantId: tenantID }),
    [apiToken, tenantID],
  );

  const load = useCallback(
    async (token = apiToken) => {
      setLoading(true);
      const errors: LoadErrors = {};
      const context = { token, tenantId: tenantID };

      const healthTask = getReadiness(context)
        .then(() => {
          setConnected(true);
          setConnectionError("");
        })
        .catch((reason: unknown) => {
          setConnected(false);
          setConnectionError(messageOf(reason));
        });
      const principalTask = getPrincipal(context)
        .then((response) => {
          setPrincipal(response);
          setPrincipalError("");
        })
        .catch((reason: unknown) => {
          setPrincipal(null);
          setPrincipalError(messageOf(reason));
        });
      const domainsTask = listDomains(context)
        .then((response) => setDomains(response.items))
        .catch((reason: unknown) => {
          errors.domains = messageOf(reason);
          setDomains([]);
        });
      const keysTask = listApiKeys(context)
        .then((response) => setKeys(response.items))
        .catch((reason: unknown) => {
          errors.keys = messageOf(reason);
          setKeys([]);
        });
      const usageTask = getUsage(context)
        .then(setUsage)
        .catch((reason: unknown) => {
          errors.usage = messageOf(reason);
          setUsage(null);
        });
      const submissionsTask = listSubmissions(context)
        .then((response) => setSubmissions(response.items))
        .catch((reason: unknown) => {
          errors.submissions = messageOf(reason);
          setSubmissions([]);
        });
      const mailboxesTask = !context.token
        ? Promise.resolve().then(() => {
            errors.mailboxes =
              "Connect a user-bound API key or configured OIDC access token to read mailboxes. The local development identity cannot access PostgreSQL-backed mailboxes.";
            setMailboxes([]);
            setMessages([]);
          })
        : listMailboxes(context)
            .then(async (response) => {
              setMailboxes(response.items);
              const mailboxID =
                response.items.find((item) => item.id === selectedMailbox)
                  ?.id ?? response.items[0]?.id;
              setSelectedMailbox(mailboxID ?? "");
              if (!mailboxID) {
                setMessages([]);
                return;
              }
              try {
                const result = await listMailboxMessages(context, mailboxID);
                setMessages(result.items);
              } catch (reason) {
                errors.mailboxes = messageOf(reason);
                setMessages([]);
              }
            })
            .catch((reason: unknown) => {
              errors.mailboxes = messageOf(reason);
              setMailboxes([]);
              setMessages([]);
            });

      await Promise.all([
        healthTask,
        principalTask,
        domainsTask,
        keysTask,
        usageTask,
        submissionsTask,
        mailboxesTask,
      ]);
      setLoadErrors(errors);
      setLoading(false);
    },
    [apiToken, selectedMailbox, tenantID],
  );

  useEffect(() => {
    const timer = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(timer);
  }, [load]);

  function applyToken(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    activityRequest.current += 1;
    setActivitySubmissionId("");
    setSubmissionAttempts([]);
    setActivityLoading(false);
    setActivityError("");
    setApiToken(tokenInput.trim());
    setTenantID(tenantInput.trim());
    setActionError("");
  }

  function clearToken() {
    activityRequest.current += 1;
    setActivitySubmissionId("");
    setSubmissionAttempts([]);
    setActivityLoading(false);
    setActivityError("");
    setTokenInput("");
    setApiToken("");
    setTenantInput("");
    setTenantID("");
    setActionError("");
  }

  async function addDomain(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setActionError("");
    try {
      await createDomain(requestContext, domainInput);
      setDomainInput("");
      await load();
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function createChallenge(domainId: string) {
    setActionError("");
    try {
      const challenge = await createDomainChallenge(requestContext, domainId);
      setChallenges((current) => ({ ...current, [domainId]: challenge }));
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function confirmOwnership(domainId: string) {
    setActionError("");
    try {
      await confirmDomainRequest(requestContext, domainId);
      await load();
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function runDomainChecks(domainId: string) {
    setActionError("");
    try {
      const result = await checkDomainRequest(requestContext, domainId);
      setDomainChecks((current) => ({
        ...current,
        [domainId]: result.results,
      }));
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function addApiKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setActionError("");
    if (keys.some((key) => key.name === keyName.trim())) {
      setActionError("An API key with this name already exists.");
      return;
    }
    try {
      const created = await createApiKey(requestContext, keyName);
      setOneTimeSecret(created.secret);
      setSecretCopied(false);
      setKeyName("");
      setView("api-keys");
      await load();
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function copyApiKey() {
    if (!oneTimeSecret) {
      return;
    }
    if (!navigator.clipboard) {
      setActionError("Clipboard access is unavailable in this browser.");
      return;
    }
    try {
      await navigator.clipboard.writeText(oneTimeSecret);
      setSecretCopied(true);
      setActionError("");
    } catch (reason) {
      setActionError(`Could not copy the API key: ${messageOf(reason)}`);
    }
  }

  function dismissApiKeySecret() {
    setOneTimeSecret("");
    setSecretCopied(false);
  }

  async function removeApiKey(keyId: string) {
    if (!window.confirm("Revoke this API key? It cannot be used again.")) {
      return;
    }
    setActionError("");
    try {
      await revokeApiKey(requestContext, keyId);
      await load();
    } catch (reason) {
      setActionError(messageOf(reason));
    }
  }

  async function selectMailbox(mailboxId: string) {
    setSelectedMailbox(mailboxId);
    if (!mailboxId) {
      setMessages([]);
      return;
    }
    try {
      const result = await listMailboxMessages(requestContext, mailboxId);
      setMessages(result.items);
      setLoadErrors((current) => ({ ...current, mailboxes: undefined }));
    } catch (reason) {
      setLoadErrors((current) => ({
        ...current,
        mailboxes: messageOf(reason),
      }));
      setMessages([]);
    }
  }

  async function loadSubmissionAttempts(submissionId: string) {
    const requestId = ++activityRequest.current;
    setActivitySubmissionId(submissionId);
    setActivityError("");
    setSubmissionAttempts([]);
    if (!submissionId) {
      setActivityLoading(false);
      return;
    }

    setActivityLoading(true);
    try {
      const result = await listSubmissionAttempts(requestContext, submissionId);
      if (requestId === activityRequest.current) {
        setSubmissionAttempts(result.items);
      }
    } catch (reason) {
      if (requestId === activityRequest.current) {
        setActivityError(messageOf(reason));
      }
    } finally {
      if (requestId === activityRequest.current) {
        setActivityLoading(false);
      }
    }
  }

  async function sendMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setActionError("");
    setSending(true);
    try {
      const idempotencyKey =
        submissionIdempotencyKey || window.crypto.randomUUID();
      setSubmissionIdempotencyKey(idempotencyKey);
      const to = recipients
        .split(/[,\n;]/)
        .map((address) => address.trim())
        .filter(Boolean);
      const created = await submitMessage(requestContext, idempotencyKey, {
        from: sender,
        to,
        subject,
        text: body,
      });
      setSubmissionIdempotencyKey("");
      setSubmissions((current) => [
        created.submission,
        ...current.filter((item) => item.id !== created.submission.id),
      ]);
      setView("send");
      await load();
    } catch (reason) {
      setActionError(messageOf(reason));
    } finally {
      setSending(false);
    }
  }

  return {
    view,
    setView,
    apiToken,
    tokenInput,
    setTokenInput,
    tenantInput,
    setTenantInput,
    principal,
    principalError,
    connected,
    connectionError,
    loading,
    domains,
    keys,
    mailboxes,
    messages,
    submissions,
    activitySubmissionId,
    submissionAttempts,
    activityLoading,
    activityError,
    usage,
    loadErrors,
    actionError,
    setActionError,
    domainChecks,
    challenges,
    domainInput,
    setDomainInput,
    keyName,
    setKeyName,
    oneTimeSecret,
    secretCopied,
    copyApiKey,
    dismissApiKeySecret,
    selectedMailbox,
    sender,
    setSender,
    recipients,
    setRecipients,
    subject,
    setSubject,
    body,
    setBody,
    sending,
    submissionIdempotencyKey,
    setSubmissionIdempotencyKey,
    verifiedDomains,
    load,
    applyToken,
    clearToken,
    addDomain,
    createChallenge,
    confirmOwnership,
    runDomainChecks,
    addApiKey,
    removeApiKey,
    selectMailbox,
    loadSubmissionAttempts,
    sendMessage,
  };
}
