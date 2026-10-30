# Phase 134 — Phishing Detection Engine

Status: **No phishing classifier or URL reputation integration implemented**

## Repository baseline

The platform has email authentication parsing and bounded MIME projections.
There is no impersonation model, URL reputation provider, safe browsing
service, or adjudicated phishing dataset. Existing authentication results are
signals and do not prove a message is benign.

## Recommended design

Combine authentication/alignment, display-name/domain similarity, URL
characteristics, known reputation, attachment scan verdicts, and user/org
policy. Return confidence and reason codes. Treat message content as untrusted;
prevent prompt injection from invoking tools or exposing unrelated data.
Isolate reputation lookups behind an approved provider with privacy,
retention, and jurisdiction review.

URL analysis must be SSRF-safe: do not fetch arbitrary message-provided
destinations from the application network. Any redirect resolution or remote
inspection belongs in a dedicated isolated service with strict egress, DNS/IP
revalidation, size/time limits, and abuse controls.

## Remaining work and why

Define threat model and provider, build a consented evaluation set, and
integrate reputation and authentication signals into reversible warnings or
quarantine. This remains open because no provider, safe inspection service,
policy, or measured false-positive target is available.

## Acceptance criteria

- Evaluations measure impersonation, URL, authentication, and language edge
  cases with false-positive/negative reporting.
- Network inspection is isolated and cannot reach private, link-local, or
  metadata networks, including through redirects or DNS rebinding.
- User-visible warnings explain signals without revealing protected
  reputation data.
- Automated actions are reversible and never solely determined by a model.
- Provider outage and stale reputation have explicit safe behavior.

## Research references

- [RFC 7489 — DMARC](https://www.rfc-editor.org/rfc/rfc7489)
- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework)
