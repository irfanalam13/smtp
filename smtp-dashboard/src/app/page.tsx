"use client";

import { FormEvent, useEffect, useState } from "react";

type Domain = { id: string; domain: string; status: string };
type ApiKey = {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  revokedAt?: string;
};
type Mailbox = { id: string; address: string; status: string };
type Message = {
  id: string;
  sender: string;
  subject: string;
  size: number;
  receivedAt: string;
};
type Check = {
  kind: string;
  status: string;
  explanation: string;
  remediation: string;
};
type ApiError = { code?: string; message?: string };

const apiBase = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBase}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      "X-Dev-Subject": "dashboard-local-user",
      ...init?.headers,
    },
  });
  if (!response.ok) {
    const error = (await response.json().catch(() => ({}))) as ApiError;
    throw new Error(error.message ?? "The API request failed");
  }
  return response.json() as Promise<T>;
}

export default function Home() {
  const [domains, setDomains] = useState<Domain[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [mailboxes, setMailboxes] = useState<Mailbox[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [checks, setChecks] = useState<Check[]>([]);
  const [usage, setUsage] = useState<{
    messages_used: number;
    message_limit: number;
  } | null>(null);
  const [domain, setDomain] = useState("");
  const [keyName, setKeyName] = useState("");
  const [secret, setSecret] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [domainResponse, keyResponse, mailboxResponse] = await Promise.all([
        request<{ items: Domain[] }>("/v1/domains"),
        request<{ items: ApiKey[] }>("/v1/api-keys"),
        request<{ items: Mailbox[] }>("/v1/mailboxes"),
      ]);
      setDomains(domainResponse.items);
      setKeys(keyResponse.items);
      const usageResponse = await request<{
        messages_used: number;
        message_limit: number;
      }>("/v1/usage");
      setUsage(usageResponse);
      setMailboxes(mailboxResponse.items);
      if (mailboxResponse.items[0]) {
        const messageResponse = await request<{ items: Message[] }>(
          `/v1/mailboxes/${mailboxResponse.items[0].id}/messages`,
        );
        setMessages(messageResponse.items);
      } else {
        setMessages([]);
      }
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not load account data",
      );
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const timer = window.setTimeout(() => void load(), 0);
    return () => window.clearTimeout(timer);
  }, []);

  async function checkDomain(id: string) {
    try {
      const result = await request<{ results: Check[] }>(
        `/v1/domains/${id}/checks`,
        { method: "POST" },
      );
      setChecks(result.results);
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Could not check DNS",
      );
    }
  }

  async function addDomain(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await request<Domain>("/v1/domains", {
        method: "POST",
        body: JSON.stringify({ domain }),
      });
      setDomain("");
      await load();
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Could not register domain",
      );
    }
  }

  async function createKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      const created = await request<{ secret: string }>("/v1/api-keys", {
        method: "POST",
        body: JSON.stringify({
          name: keyName,
          scopes: ["domains:read", "delivery:read"],
        }),
      });
      setSecret(created.secret);
      setKeyName("");
      await load();
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Could not create API key",
      );
    }
  }

  async function revokeKey(id: string) {
    if (!window.confirm("Revoke this API key? It cannot be used again."))
      return;
    try {
      await request(`/v1/api-keys/${id}/revoke`, { method: "POST" });
      await load();
    } catch (reason) {
      setError(
        reason instanceof Error ? reason.message : "Could not revoke API key",
      );
    }
  }

  return (
    <main className="shell">
      <header className="header">
        <div>
          <p className="eyebrow">SMTP Platform · local development identity</p>
          <h1>Control plane</h1>
          <p className="muted">
            Manage verified sending domains and scoped API keys.
          </p>
        </div>
        <span className="identity">dashboard-local-user</span>
      </header>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {secret && (
        <section className="secret" aria-live="polite">
          <strong>Copy this API key now</strong>
          <p>This secret is shown once and is not stored in browser storage.</p>
          <code>{secret}</code>
          <button type="button" onClick={() => setSecret(null)}>
            Dismiss secret
          </button>
        </section>
      )}
      <div className="grid">
        <section className="card">
          <h2>Sending domains</h2>
          <p className="muted">
            Ownership verification is required before a domain can send.
          </p>
          <form onSubmit={addDomain} className="form">
            <label htmlFor="domain">Domain</label>
            <input
              id="domain"
              value={domain}
              onChange={(event) => setDomain(event.target.value)}
              placeholder="example.com"
              required
            />
            <button type="submit">Register domain</button>
          </form>
          {loading ? (
            <p className="muted">Loading domains…</p>
          ) : domains.length === 0 ? (
            <p className="empty">No domains registered.</p>
          ) : (
            <ul className="list">
              {domains.map((item) => (
                <li key={item.id}>
                  <span>
                    {item.domain}
                    <small>
                      {checks
                        .filter(
                          (check) => check.status !== "pass" && check.kind,
                        )
                        .map((check) => `${check.kind}: ${check.status}`)
                        .join(" · ")}
                    </small>
                  </span>
                  <span>
                    <span className={`status ${item.status}`}>
                      {item.status.replace("_", " ")}
                    </span>
                    <button
                      type="button"
                      onClick={() => void checkDomain(item.id)}
                    >
                      Check DNS
                    </button>
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="card">
          <h2>API keys</h2>
          <p className="muted">
            Secrets are high-entropy, scoped, and displayed only at creation.
          </p>
          <form onSubmit={createKey} className="form">
            <label htmlFor="key-name">Key name</label>
            <input
              id="key-name"
              value={keyName}
              onChange={(event) => setKeyName(event.target.value)}
              placeholder="Local integration"
              required
            />
            <button type="submit">Create API key</button>
          </form>
          {loading ? (
            <p className="muted">Loading keys…</p>
          ) : keys.length === 0 ? (
            <p className="empty">No API keys created.</p>
          ) : (
            <ul className="list">
              {keys.map((item) => (
                <li key={item.id}>
                  <span>
                    <strong>{item.name}</strong>
                    <small>
                      {item.prefix} · {item.scopes.join(", ")}
                    </small>
                  </span>
                  <button type="button" onClick={() => void revokeKey(item.id)}>
                    Revoke
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
      <section className="card note">
        <h2>Delivery activity</h2>
        <p className="muted">
          Delivery activity will appear here when backed by persisted queue
          attempt data. No fabricated analytics are shown.
        </p>
      </section>
      <section className="card note">
        <h2>Usage</h2>
        <p className="muted">
          {usage
            ? `${usage.messages_used} of ${usage.message_limit} message submissions used in the current one-hour window.`
            : "Loading usage…"}
        </p>
        {checks.length > 0 && (
          <ul className="list">
            {checks.map((check) => (
              <li key={check.kind}>
                <span>
                  <strong>{check.kind}</strong>
                  <small>
                    {check.explanation} {check.remediation}
                  </small>
                </span>
                <span className={`status ${check.status}`}>{check.status}</span>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="card note">
        <h2>Mailbox</h2>
        {mailboxes.length === 0 ? (
          <p className="empty">No mailboxes are configured for this tenant.</p>
        ) : messages.length === 0 ? (
          <p className="empty">This mailbox has no messages.</p>
        ) : (
          <ul className="list">
            {messages.map((item) => (
              <li key={item.id}>
                <span>
                  <strong>{item.subject || "(no subject)"}</strong>
                  <small>
                    From {item.sender} · {item.size} bytes
                  </small>
                </span>
                <time dateTime={item.receivedAt}>
                  {new Date(item.receivedAt).toLocaleString()}
                </time>
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
