"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { requestJson } from "@/lib/api/json-client";
import type { RequestContext } from "@/lib/api/json-client";

type AuthSession = {
  id: string;
  tenant_id: string;
  user_agent: string;
  created_at: string;
  last_used_at: string;
  idle_expires_at: string;
  expires_at: string;
  is_current?: boolean;
};

type SessionList = { items: AuthSession[] };

export function SessionManagement({
  context,
}: {
  context: RequestContext;
}) {
  const router = useRouter();
  const [sessions, setSessions] = useState<AuthSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  const loadSessions = useCallback(
    async (signal?: AbortSignal) =>
      requestJson<SessionList>(
        "/v1/auth/sessions",
        context,
        { signal },
      ),
    [context],
  );

  useEffect(() => {
    const controller = new AbortController();
    void loadSessions(controller.signal)
      .then((response) => {
        if (!controller.signal.aborted) {
          setSessions(response.items);
        }
      })
      .catch((reason: unknown) => {
        if (!controller.signal.aborted) {
          setError(
            reason instanceof Error
              ? reason.message
              : "Active sessions could not be loaded.",
          );
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });
    return () => controller.abort();
  }, [loadSessions]);

  async function revokeSession(session: AuthSession) {
    setBusy(session.id);
    setError("");
    setMessage("");
    try {
      await requestJson(
        `/v1/auth/sessions/${encodeURIComponent(session.id)}`,
        context,
        { method: "DELETE" },
      );
      if (session.is_current) {
        router.replace("/login?error=session_expired");
        return;
      }
      setMessage("The selected session was revoked.");
      const response = await loadSessions();
      setSessions(response.items);
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "The session could not be revoked.",
      );
    } finally {
      setBusy("");
    }
  }

  async function logoutAll() {
    setBusy("all");
    setError("");
    try {
      await requestJson("/v1/auth/logout-all", context, { method: "POST" });
      router.replace("/login?error=session_expired");
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Sessions could not be revoked.",
      );
      setBusy("");
    }
  }

  async function logoutThisDevice() {
    setBusy("current");
    setError("");
    try {
      await requestJson("/v1/auth/logout", context, { method: "POST" });
      router.replace("/login");
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Sign-out failed.",
      );
      setBusy("");
    }
  }

  return (
    <section
      aria-labelledby="active-sessions-heading"
      className="mt-5 rounded-xl border border-zinc-200 bg-white p-4 sm:p-5"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2
            className="text-sm font-semibold text-zinc-900"
            id="active-sessions-heading"
          >
            Active sessions
          </h2>
          <p className="mt-1 text-xs leading-5 text-zinc-600">
            Review browsers signed in to your account and revoke access.
          </p>
        </div>
        <button
          className="button button-danger-quiet"
          disabled={busy !== ""}
          onClick={() => void logoutAll()}
          type="button"
        >
          Sign out all devices
        </button>
      </div>

      {error && (
        <p className="notice notice-error mt-3" role="alert">
          {error}
        </p>
      )}
      {message && (
        <p className="notice notice-success mt-3" role="status">
          {message}
        </p>
      )}
      {loading ? (
        <p aria-live="polite" className="mt-4 text-sm text-zinc-600" role="status">
          Loading active sessions…
        </p>
      ) : sessions.length === 0 ? (
        <p className="mt-4 text-sm text-zinc-600">No active sessions found.</p>
      ) : (
        <ul className="mt-4 divide-y divide-zinc-100">
          {sessions.map((session) => (
            <li
              className="flex flex-wrap items-center justify-between gap-3 py-3 first:pt-0 last:pb-0"
              key={session.id}
            >
              <div className="min-w-0">
                <p className="break-words text-sm font-medium text-zinc-800">
                  {session.user_agent || "Unknown browser"}
                  {session.is_current ? (
                    <span className="ml-2 rounded-full bg-emerald-50 px-2 py-0.5 text-xs text-emerald-800">
                      This device
                    </span>
                  ) : null}
                </p>
                <p className="mt-1 text-xs text-zinc-500">
                  Last active{" "}
                  <time dateTime={session.last_used_at}>
                    {new Date(session.last_used_at).toLocaleString()}
                  </time>
                  {" · "}
                  Expires{" "}
                  <time dateTime={session.expires_at}>
                    {new Date(session.expires_at).toLocaleString()}
                  </time>
                </p>
              </div>
              <button
                className="button button-quiet"
                disabled={busy !== ""}
                onClick={() => void revokeSession(session)}
                type="button"
              >
                {busy === session.id ? "Revoking…" : "Revoke"}
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-4 border-t border-zinc-100 pt-4">
        <button
          className="button button-quiet"
          disabled={busy !== ""}
          onClick={() => void logoutThisDevice()}
          type="button"
        >
          Sign out this device
        </button>
      </div>
    </section>
  );
}
