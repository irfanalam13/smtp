# Phase 92 — Marketplace and Extensions

Status: **Not implemented; extension execution intentionally not designed as in-process plugins**

## Repository baseline

The architecture uses a small number of services with explicit ownership.
There is no marketplace catalog, plugin loader, extension permission model,
installation lifecycle, tenant app store, or sandbox runtime.

## Recommended extension model

Start with reviewed, declarative integrations and outbound webhooks, not
arbitrary tenant-supplied code. If extensions are later approved, run them
outside control-plane and SMTP processes behind a narrow, versioned,
capability-based API. Treat package metadata, manifests, callbacks, and
updates as untrusted. Require signing/provenance, dependency and vulnerability
scanning, least-privilege scopes, tenant consent, resource quotas, version
pinning, revocation, kill switches, audit, and reproducible rollback.

Sandboxing is a deployment/security architecture decision: a container alone
is not a sufficient hostile-code boundary. Do not run plugin code with
database credentials, host mounts, SMTP relay authority, or ambient network
access. Marketplace review is operational and does not guarantee safety.

## Remaining work and why

Gather use cases and threat-model the extension supply chain; define supported
manifest/permissions, signing authority, approval process, execution isolation,
data egress, billing, update/rollback, incident response, and operator
ownership. Implement a catalog of declarative integrations first. Only add
runtime extensions after independent security review and sandbox benchmarks.

This remains open because no customer extension requirement, sandbox
technology, reviewer/operator, or support model exists. Executing third-party
code in a multi-tenant mail platform can expose credentials, message contents,
and shared infrastructure.

## Acceptance criteria

- Extensions cannot access data or capabilities beyond explicit tenant grants.
- Runtime isolation is tested against filesystem, network, process, memory,
  CPU, and cross-tenant escape attempts.
- Signing, provenance, dependency scanning, version pinning, revocation, and
  rollback are enforced.
- Installation/permission changes and data egress are consented, audited, and
  immediately revocable.
- No extension runs on SMTP acceptance or delivery critical paths.

## Research references

- [Phase 85 Public Integrations](phase-85-public-integrations.md)
- [OWASP Software Supply Chain Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Software_Supply_Chain_Security_Cheat_Sheet.html)
- [NIST Secure Software Development Framework (SP 800-218)](https://csrc.nist.gov/pubs/sp/800/218/final)
