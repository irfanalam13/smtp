# Phase 35 — DMARC Engine

DMARC follows the current RFC 9989, with reporting specified by RFCs 9990 and
9991. A message passes when an aligned authenticated SPF or DKIM result passes.
Policy includes `p`, `sp`, `np`, `adkim`, `aspf`, PSD tree-walk discovery,
`t=y`, and authorized reporting URIs. RFC 9989 removed the legacy `pct` tag.

Inbound enforcement delegates policy discovery, strict/relaxed alignment,
public-suffix-domain handling, `p`/`sp`/`np`, and test-mode disposition to the
`mail-auth` implementation. SMTP rejects failing reject policy, quarantines
failing quarantine policy, and defers temporary DNS errors. The old core helper
remains a simplified reference function and is not used for enforcement.

RFC 9990 report events are collected without message bodies or recipient
addresses, authorized destinations are checked before collection, daily reports
are emitted as bounded gzip XML batches, authorized destinations are rechecked
before dispatch, and the report worker enqueues them through the regular
outbound delivery path. The ingestion command accepts
bounded XML/gzip, rejects DTDs, deduplicates Report-IDs, stores normalized data,
and applies 90-day expiry. RFC 9991 failure reporting remains disabled because
it contains sensitive message evidence and has no opt-in controls or privacy
review.
