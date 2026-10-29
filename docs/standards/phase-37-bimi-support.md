# Phase 37 — Brand Indicators for Message Identification

## Standards baseline

BIMI is a community specification and deployment profile, not an SMTP
authentication mechanism. Logo display depends on successful DMARC enforcement
and receiver policy; certificate/authority requirements vary by mailbox
provider. A BIMI record or HTTPS URL alone does not establish that a logo is
verified or eligible for display.

## Implemented foundation

The core parses `v`, `l`, and optional `a` tags, requires `BIMI1`, and rejects
non-HTTPS URLs, empty hosts, embedded credentials, and control/whitespace.
This only validates record syntax and basic URL shape.

## Production requirements

- Resolve the correct BIMI DNS name with DNSSEC-aware policy.
- Fetch logos under strict SSRF, size, redirect, media-type, and timeout limits.
- Validate SVG Tiny PS and certificate/VMC/CMC evidence using current provider
  requirements.
- Separate `record present`, `logo fetched`, `authority validated`, and
  `eligible to display` results.
- Cache safely and support revocation and periodic revalidation.

