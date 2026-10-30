# Phase 136 — URL Reputation and Link Analysis

Status: **Offline textual URL-reference counts only; no reputation service or link fetching**

## Repository baseline

The Phase 132 extractor counts HTTP, HTTPS, and `www.` references from parsed
text without retaining destinations or making network requests. There is no
reputation provider, redirect resolver, sandbox, or link-warning UI.

## Recommended design

Normalize and deduplicate URL observations in a separate privacy-reviewed
component. Use a provider contract with data minimization, caching limits,
freshness, tenant isolation, and explicit sharing/retention. Treat short links
and redirects as untrusted; remote resolution must run in a dedicated egress
sandbox that blocks private/reserved destinations, revalidates DNS at every
hop, limits redirects/bytes/time, and never forwards ambient credentials.

## Remaining work and why

Select an approved reputation source, build safe isolated URL inspection,
define privacy/retention and warning semantics, and validate false positives.
This remains open because external lookups reveal user activity and arbitrary
URL fetching creates a significant SSRF and abuse boundary.

## Acceptance criteria

- No application service directly fetches message-provided URLs.
- Lookup service enforces destination/IP restrictions, redirect and resource
  bounds, and isolated egress.
- Reputation data is versioned/freshness-aware and provider failure is explicit.
- URL data is not exposed as high-cardinality metrics or stored beyond policy.
- Warnings are explainable and do not imply that an unlisted URL is safe.

## Research references

- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- [Phase 132 Email Feature Extraction](phase-132-email-feature-extraction.md)
