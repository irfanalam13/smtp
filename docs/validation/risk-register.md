# Phase 10 risk register

| ID | Severity | Finding | Evidence | Mitigation/status |
|---|---|---|---|---|
| R-001 | High | Rust dependency audit reports `idna 0.4.0` advisory RUSTSEC-2024-0421 | `cargo audit` | Replace/upgrade the resolver dependency before public production delivery |
| R-002 | Medium | RustSec reports `rsa 0.9.10` Marvin timing advisory RUSTSEC-2023-0071 with no fixed upgrade | `cargo audit` | Trace target-specific reachability, remove/isolate affected capability, or obtain a documented exception |
| R-003 | Medium | `trust-dns-proto 0.23.2` is unmaintained | `cargo audit` | Migrate to maintained Hickory dependency before production release |
| R-004 | High | Go control-plane state and API-key state are in memory | Repository inspection and Phase 06/08 status | Implement PostgreSQL repositories before multi-instance or public deployment |
| R-005 | High | Production OIDC, service identity, and durable secret integration are unavailable | Phase 07 security documentation | Select and integrate approved identity/secret providers before public deployment |
| R-006 | High | No centralized/distributed telemetry backend or authenticated dashboard deployment | Phase 09 inspection | Select deployment telemetry stack and access controls before production operations |
| R-007 | Medium | Performance evidence is only a local sequential smoke baseline | Performance report | Run representative concurrency/load/failure validation in Phase 10 deployment environment |
| R-008 | Medium | Gitleaks and remote CI/branch protection were not locally verified | Tool/repository access limitation | Run security workflow in CI and inspect repository settings before release |
| R-009 | Medium | Event outbox, durable idempotency, and internal service communication are not implemented | Phase 08 documentation | Implement only when real multi-service consumers and durable storage exist |
| R-010 | Low | Existing Compose volume credentials differed from temporary validation credentials | Local environment observation | Preserved volume; disposable container used. Document local credential reset procedure if needed |

## Release blockers

R-001, R-004, R-005, and R-006 are release blockers for a public production
email service. R-007 and R-008 must be resolved before claiming production
capacity or repository governance readiness.
