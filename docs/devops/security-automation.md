# Security automation

## Checks

- `cargo audit --locked` scans the committed Rust dependency graph using the
  RustSec advisory database.
- Dependency Review runs on pull requests and fails on high or critical
  dependency findings.
- Gitleaks scans repository history and pull-request changes. A finding is
  potentially compromised until the owner confirms otherwise; remove, revoke,
  and rotate the credential rather than merely deleting the file.
- The workflow guardrail checks require explicit permissions and reject
  `pull_request_target` and dynamic shell evaluation in repository workflows.
- Clippy runs with warnings denied as part of CI.

These checks reduce risk but do not prove that the platform is secure. Findings
must be triaged with reachability, exploitability, affected versions, and
available fixes.

The current locked dependency graph is not a clean security baseline:

- `RUSTSEC-2024-0421` affects `idna 0.4.0`, pulled through the
  `trust-dns` resolver used by `mail-delivery`; upgrade or replace the resolver
  before a production release.
- `RUSTSEC-2023-0071` affects `rsa 0.9.10` and has no fixed upgrade reported
  by RustSec; identify the target-specific dependency path and remove or
  isolate the affected capability before release.
- `RUSTSEC-2025-0017` marks `trust-dns-proto 0.23.2` unmaintained. Migration
  to the maintained Hickory project is a planned remediation.

The security workflow intentionally fails on these findings. They are not
silenced with an audit ignore file.

## Severity policy

Critical and high findings block merge or release unless an explicit,
time-bounded exception is approved by maintainers. Medium findings require a
tracked remediation or documented rationale. Low findings should be fixed in
normal maintenance. A scanner outage is not a pass: record the unavailable
check and rerun it before release.

This repository currently has no private security mailbox or advisory URL.
Follow [SECURITY.md](../../SECURITY.md) for reporting and do not put
exploitable details in public issues.

## Workflow security

Actions run on ephemeral hosted runners. Untrusted pull requests receive no
publishing credentials. Release publication is limited to the release job,
which uses the ephemeral `GITHUB_TOKEN` and a trusted version tag. Third-party
actions are currently referenced by maintained major versions; pin them to
reviewed commit SHAs when repository governance adopts an action pinning
workflow.
