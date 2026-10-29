# Phase 35 — DMARC Engine

DMARC follows RFC 7489. A message passes when an aligned authenticated SPF or
DKIM result passes. Policy records define `p`, optional `sp`, reporting URIs,
and sampling percentage; enforcement must be tenant/domain policy controlled
and observable.

The core foundation evaluates strict domain alignment and returns none,
quarantine, or reject outcomes. Organizational-domain relaxed alignment,
policy discovery, `sp`, `pct`, aggregate/forensic reporting, and enforcement
hooks remain integration work.

