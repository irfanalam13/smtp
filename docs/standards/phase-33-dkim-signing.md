# Phase 33 — DKIM Signing

DKIM follows RFC 6376. Signing covers a canonicalized body and selected headers,
and private keys must remain in a protected key service or tightly controlled
process. Relaxed canonicalization normalizes header whitespace but does not
lowercase message body content.

The core foundation implements simple/relaxed canonicalization and an injected
signer boundary that emits a structured `DKIM-Signature` value. RSA key
loading, selector rotation, DNS publication, body-length policy, and key
custody remain deployment adapters.

