# Phase 36 — Authenticated Received Chain

## Standards baseline

ARC is specified by [RFC 8617](https://www.rfc-editor.org/rfc/rfc8617), an
Experimental protocol, not an Internet Standards Track protocol. ARC defines
ordered `ARC-Authentication-Results`, `ARC-Message-Signature`, and
`ARC-Seal` sets that carry authentication assessments through intermediaries.
Structural validity alone is not cryptographic validity, and an ARC pass is
not a blanket reason to ignore local DMARC policy.

## Implemented foundation

The core validates contiguous ARC instance numbering and required set fields.
Cryptographic verification is injected through `ArcVerifier`. The combined
authentication report preserves the ARC result but does not automatically
override DMARC disposition.

## Production requirements

- Parse and serialize ARC header fields according to RFC 8617.
- Implement ARC-specific canonicalization, signing, and verification.
- Validate chain links, `cv` semantics, selector keys, and header ordering.
- Keep ARC trust decisions as explicit local policy.
- Add fixtures for malformed sets, broken seals, and forwarding/list scenarios.

