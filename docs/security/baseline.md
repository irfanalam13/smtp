# Security baseline

- Bind local database ports to loopback only.
- Keep credentials in local environment files or a secret manager; never commit them.
- Do not write message contents, passwords, tokens, or credentials to logs.
- Review dependency and CI action updates before merging.
- Run migrations against disposable databases in CI.
- The SMTP listener binds to loopback by default and has bounded connections,
  command lines, recipients, message data, and idle timeouts. It does not
  advertise TLS or AUTH. Do not expose it publicly without a reviewed MTA,
  TLS, authentication, abuse-control, and network-boundary design.
- In the default protocol-test configuration, message bodies are retained only
  in memory. Explicit queue or inbound integrations persist data in
  PostgreSQL and must follow the retention and access controls documented by
  the deployment.
- Milestone 2 stores bounded test message bytes in PostgreSQL. Queue diagnostics
  use stable categories and never contain bodies, credentials, or private
  content.
- Queue integration tests require an explicitly named disposable database and
  must not target a developer's normal database.
- Outbound SMTP delivery is disabled by default and must never be enabled as a
  side effect of starting the ingress or queue worker.
- MX results are resolved to addresses and filtered before connecting. Private,
  loopback, link-local, multicast, unspecified, documentation, and reserved
  ranges are rejected unless explicit loopback fixture mode is enabled.
- A validated destination IP is used for the connection to prevent DNS
  rebinding between validation and connect. STARTTLS certificate validation is
  tied to the MX hostname, and advertised STARTTLS failures never downgrade.
- SMTP replies, message sizes, MX candidates, and connection attempts are
  bounded. Delivery diagnostics must not include message bodies or credentials.
- The Milestone 4 control plane uses a development-only identity adapter when
  `CONTROL_PLANE_DEV_MODE=true`; production OIDC validation is intentionally
  deferred until issuer and audience configuration exists. The local header
  must never be accepted in production.
- Tenant IDs come from the authenticated principal, not request payloads.
  Domains and API keys are checked at the service boundary before every
  operation. Authorization is default-deny and resource tenant scope must
  match the authenticated identity. API-key secrets are shown once, represented
  as service identities, and never returned by list calls. The development
  identity adapter is rejected when `APP_ENVIRONMENT` is production.
- Inbound reception is opt-in and must be deployed behind an explicitly
  configured MTA/network boundary. It does not advertise STARTTLS or AUTH.
- Inbound routing requires a verified domain and active explicitly configured
  mailbox; arbitrary recipients are never assigned to a tenant.
- Raw inbound bytes are bounded and committed with routing metadata in one
  transaction. Message bodies are not logged or returned in list responses,
  and the dashboard does not render email HTML.
- DNS diagnostics use an injected resolver in tests, bounded timeouts, and
  tenant-scoped rate limits. Tenant-provided DNS servers or arbitrary network
  targets are never accepted.
- SPF, DMARC, and MX results are advisory and never authorize a foreign sender,
  alter DNS, or claim inbox placement. DKIM signing is not implemented.
- The local bootstrap and runner scripts are development tools. They must not
  be used as a production supervisor, and `deploy-preflight.sh` validates but
  does not deploy.
