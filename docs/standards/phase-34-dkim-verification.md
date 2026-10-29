# Phase 34 — DKIM Verification

Verification must parse the signature, retrieve the selector TXT record, apply
the declared canonicalization, verify the cryptographic signature, and expose
pass/fail/temperror/permerror outcomes without silently treating failure as
success.

The core foundation reconstructs canonicalized input and delegates cryptographic
verification to an injected verifier. DNS selector retrieval, RSA/Ed25519
algorithm support, duplicate-header selection, key revocation, and result
reporting remain integration work.

