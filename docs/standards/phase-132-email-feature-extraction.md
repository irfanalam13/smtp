# Phase 132 — Email Feature Extraction Engine

Status: **Offline aggregate feature extractor implemented; richer, evaluated projections pending**

## Repository baseline and code slice

`shared-types::ai_features::extract_email_features` accepts a parsed MIME
projection and returns byte/word counts, counts of HTTP/HTTPS and bare-web
references, text/HTML part counts, attachment count, executable-like
attachment count, and parser warning count. It does not retain or return
addresses, subjects, filenames, URLs, bodies, or attachment bytes. This
feature set is intentionally heuristic and does not determine message safety.

## Recommended design

Extend features only through reviewed versioned schemas with provenance,
language/charset handling, authentication results, sender/domain reputation
inputs, attachment scan results, and normalized URL observations. Preserve the
canonical message unchanged; parse and decode under established MIME resource
limits. Keep raw and derived-data retention separate. Do not fetch a URL or
open an attachment during feature extraction.

## Remaining work and why

Add a versioned feature contract, RFC fixture and fuzz coverage, CPU/memory
budgets, feature quality tests, and asynchronous integration with durable
mailbox/queue flows. This remains incomplete because current input is a local
parser projection and has no pipeline, dataset, retention, or feature
compatibility management.

## Acceptance criteria

- Feature schema is versioned, deterministic for a fixed parsed input, and
  contains no unnecessary direct identifiers or raw content.
- Limits from MIME parsing remain effective for nested parts and decoded data.
- Malformed input and oversized messages have explicit non-success outcomes.
- Feature changes have representative offline fixtures and compatibility tests.
- No feature-extraction path performs network access or attachment execution.

## Research references

- [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322)
- [RFC 2045](https://www.rfc-editor.org/rfc/rfc2045)
- [Phase 47 MIME Parsing](phase-47-mime-parsing.md)
