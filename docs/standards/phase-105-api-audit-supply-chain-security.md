# Phase 105 — API, Audit, and Supply-Chain Security

Status: **Several API/security checks and CI foundations exist; comprehensive hardening and durable audit evidence pending**

## Repository baseline

The API has authentication/authorization tests, bounded request handling,
health and metrics endpoints, and local dependency/build checks. There is an
audit table and security documentation, but durable runtime audit integration,
complete API abuse controls, enterprise vulnerability-management operations,
and verified supply-chain attestations are not established. Release readiness
records unresolved dependency findings and missing production operational
evidence.

The API now also sets `Cache-Control: no-store`, a restrictive
`Content-Security-Policy`, `Referrer-Policy: no-referrer`,
`X-Content-Type-Options: nosniff`, and `X-Frame-Options: DENY` on responses,
and limits request header size. These are defense-in-depth HTTP controls, not
an API security audit or complete abuse-control system.

The security workflow now runs pinned `govulncheck` against reachable Go
vulnerabilities. The Go toolchain was upgraded to 1.26.10 and
`golang.org/x/text` to 0.41.0 after the scan found applicable advisories;
the follow-up scan reported no known Go vulnerabilities. This does not resolve
the separately tracked RustSec findings or replace broader supply-chain
attestation.

## Recommended design

Threat-model each public API/SMTP boundary. Add schema validation, bounded
payloads, request/connection/time budgets, tenant-aware quotas, abuse and
credential-stuffing protections, safe error responses, and rate limits at
both edge and application layers. Produce auditable security events in a
durable restricted sink with redaction, integrity, retention, access review,
and reliable delivery. Pin/review dependencies, generate an SBOM, scan
artifacts, sign build outputs, protect CI credentials, and define
vulnerability response SLAs. Scanning is evidence, not proof of absence.

## Remaining work and why

Define API threat model, edge provider, logging/audit retention, vulnerability
SLAs, CI trust model, signing authority, SBOM format, and security test scope.
Implement missing controls after the real deployment path and business limits
are selected. This remains open because edge infrastructure, durable audit
ownership, release signing, and security operations are not configured.

## Acceptance criteria

- Security-relevant changes and denials are durably audited with actor,
  tenant, action, outcome, and correlation ID, while secrets/content are
  excluded.
- API abuse, resource consumption, authentication, and object-authorization
  controls have automated negative tests.
- Release artifacts have provenance, SBOM, signature verification, and
  dependency/license review.
- CI tokens are least-privilege, short-lived where possible, and isolated
  from untrusted pull-request code.
- Vulnerabilities have triage, owner, remediation deadline, and verified
  closure evidence.

## Research references

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [NIST Secure Software Development Framework (SP 800-218)](https://csrc.nist.gov/pubs/sp/800/218/final)
- [Phase 90 Audit and Compliance](phase-90-audit-compliance.md)
- [Security testing policy](../security/security-testing.md)
