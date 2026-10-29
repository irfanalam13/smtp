# Phase 31 — DNS Integration

## Research baseline

SMTP delivery uses MX records with A/AAAA fallback behavior. SPF, DKIM, and
DMARC use TXT records; PTR is useful for operational identity and policy
signals but is not an authorization result by itself. DNSSEC validation must
be distinguished from ordinary DNS answers, and caches must honor TTLs and
negative caching.

## Implemented foundation

The existing delivery resolver performs timeout-bounded MX and address lookup
with public-destination filtering. The shared core now provides normalized
typed record caching for A, AAAA, MX, PTR, and TXT records.

## Production requirements

- Add TTL-aware positive and negative caching.
- Expose DNSSEC authenticated-data and bogus states.
- Enforce resolver timeouts, query limits, rebinding protections, and metrics.
- Add A/AAAA fallback only according to the SMTP delivery policy.

