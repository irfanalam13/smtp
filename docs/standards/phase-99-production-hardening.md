# Phase 99 — Production Hardening

Status: **Local validation and security foundations exist; production hardening evidence incomplete**

## Repository baseline

The repository has CI checks, Rust/Go/dashboard tests, API security tests,
migrations, local backup/restore procedures, health/metrics endpoints, and a
risk register. Existing release-readiness documentation explicitly says the
platform is not production-ready and lists unresolved dependency findings,
missing production identity/secrets/telemetry, limited performance evidence,
and absent deployment governance.

## Hardening program

Treat production hardening as an evidence program, not a single checklist or
code freeze. Maintain threat models, dependency/SBOM review, secret scanning,
SAST/DAST where appropriate, fuzzing of parsers/protocols, independent
penetration testing, incident exercises, and vulnerability response.
Security tests must be authorized and isolated. Build chaos experiments from
documented failure hypotheses and bounded blast radius; do not inject faults
into production without an approved experiment plan.

Validate backup/restore and disaster recovery with recurring exercises.
Establish SLOs, alert ownership, runbooks, capacity/error budgets, queue lag,
mail abuse/reputation responses, safe rollout/rollback, and data lifecycle
controls. High-volume SMTP services need abuse, open-relay, credential,
reputation, bounce, and destination-throttling tests in addition to generic
web-app testing.

## Remaining work and why

Resolve tracked security/dependency findings; select production runtime and
providers; conduct authorized penetration and privacy reviews; run
representative load, failover, restore, chaos, protocol interoperability,
and abuse-resilience tests; produce signed artifacts/SBOM; and close release
blockers with named owners and evidence.

This remains open because the current evidence is primarily local and
component-level. Passing unit tests or a production build does not prove
network, deployment, operational, security, or recovery behavior at scale.
Some requirements require an external auditor, provider, operator, or
customer environment.

## Acceptance criteria

- Release blockers have owner, severity, mitigation, due date, and verified
  evidence; no unreviewed critical/high issue is silently waived.
- Authorized independent security testing and remediation are complete.
- Restore/failover and incident exercises meet approved objectives.
- Load, abuse, interoperability, migration, and upgrade tests meet explicit
  targets on production-like infrastructure.
- Monitoring, paging, runbooks, rollback, and on-call ownership are verified.

## Research references

- [Current release-readiness assessment](../validation/release-readiness.md)
- [Risk register](../validation/risk-register.md)
- [NIST Secure Software Development Framework (SP 800-218)](https://csrc.nist.gov/pubs/sp/800/218/final)
- [Google SRE Workbook](https://sre.google/workbook/table-of-contents/)
