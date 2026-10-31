"use client";

import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { RequestContext } from "@/lib/api/json-client";
import {
  createSMTPCredential,
  listSMTPCredentials,
  revokeSMTPCredential,
  rotateSMTPCredential,
} from "../services/smtp-credentials-api";
import type {
  CreatedSMTPCredential,
  SMTPCredential,
} from "../types/smtp-credentials";

function defaultExpiry(): string {
  const date = new Date();
  date.setMonth(date.getMonth() + 3);
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function expiryISO(value: string): string {
  const date = new Date(value);
  if (!value || Number.isNaN(date.getTime()) || date <= new Date()) {
    throw new Error("Choose an expiry time in the future.");
  }
  if (date > new Date(Date.now() + 365 * 24 * 60 * 60 * 1000)) {
    throw new Error("SMTP credentials can expire no more than one year ahead.");
  }
  return date.toISOString();
}

export function useSMTPCredentials(context: RequestContext, active: boolean) {
  const [credentials, setCredentials] = useState<SMTPCredential[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [username, setUsername] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [busyAction, setBusyAction] = useState("");
  const [oneTimeCredential, setOneTimeCredential] =
    useState<CreatedSMTPCredential | null>(null);
  const [secretCopied, setSecretCopied] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const response = await listSMTPCredentials(context);
      setCredentials(response.items);
    } catch (reason) {
      setCredentials([]);
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not load SMTP credentials.",
      );
    } finally {
      setLoading(false);
    }
  }, [context]);

  useEffect(() => {
    if (!active) return;
    const task = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(task);
  }, [active, load]);

  useEffect(() => {
    if (!active) return;
    const task = window.setTimeout(() => {
      setExpiresAt((current) => current || defaultExpiry());
    }, 0);
    return () => window.clearTimeout(task);
  }, [active]);

  async function run(action: string, operation: () => Promise<void>) {
    if (busyAction) return;
    setBusyAction(action);
    setError("");
    try {
      await operation();
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "The request failed.",
      );
    } finally {
      setBusyAction("");
    }
  }

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalized = username.trim().toLowerCase();
    if (!/^[a-zA-Z0-9._+@-]{1,128}$/.test(normalized)) {
      setError("Use 1–128 letters, numbers, or . _ + @ - in the username.");
      return;
    }
    let expiry: string;
    try {
      expiry = expiryISO(expiresAt);
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Invalid expiry time.",
      );
      return;
    }
    await run("create", async () => {
      const created = await createSMTPCredential(context, normalized, expiry);
      setOneTimeCredential(created);
      setSecretCopied(false);
      setUsername("");
      await load();
    });
  }

  async function rotate(credentialId: string) {
    let expiry: string;
    try {
      expiry = expiryISO(expiresAt);
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Invalid expiry time.",
      );
      return;
    }
    await run(`rotate:${credentialId}`, async () => {
      const rotated = await rotateSMTPCredential(context, credentialId, expiry);
      setOneTimeCredential(rotated);
      setSecretCopied(false);
      await load();
    });
  }

  async function revoke(credentialId: string) {
    await run(`revoke:${credentialId}`, async () => {
      await revokeSMTPCredential(context, credentialId);
      setOneTimeCredential(null);
      await load();
    });
  }

  async function copySecret() {
    if (!oneTimeCredential) return;
    try {
      await navigator.clipboard.writeText(
        `Username: ${oneTimeCredential.authcid}\nPassword: ${oneTimeCredential.password}`,
      );
      setSecretCopied(true);
    } catch {
      setError(
        "Clipboard access is unavailable. Copy the username and password manually.",
      );
    }
  }

  return {
    credentials,
    loading,
    error,
    username,
    setUsername,
    expiresAt,
    setExpiresAt,
    busyAction,
    oneTimeCredential,
    secretCopied,
    create,
    rotate,
    revoke,
    copySecret,
    dismissSecret: () => {
      setOneTimeCredential(null);
      setSecretCopied(false);
    },
    reload: load,
  };
}
