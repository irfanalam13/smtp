# Phase 34 — DKIM Verification

Verification must parse the signature, retrieve the selector TXT record, apply
the declared canonicalization, verify the cryptographic signature, and expose
pass/fail/temperror/permerror outcomes without silently treating failure as
success.

Inbound enforcement delegates signature parsing, bottom-up signed-header
selection, body canonicalization and `l=` handling, DNS selector retrieval,
key checks, RSA-SHA256/Ed25519-SHA256 verification, and result classes to
`mail-auth`. RSA-SHA1 verification output is explicitly excluded from DMARC
alignment. The adapter caps messages at four DKIM signatures. RFC-indexed
application fixtures, key rotation/revocation interoperability fixtures, and
real fixture-domain evidence remain outstanding.

