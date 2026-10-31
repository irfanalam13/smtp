# Phase 33 — DKIM Signing

DKIM follows RFC 6376. Signing covers a canonicalized body and selected headers,
and private keys must remain in a protected key service or tightly controlled
process. Relaxed canonicalization normalizes header whitespace but does not
lowercase message body content.

Outbound SMTP delivery signs with Ed25519-SHA256 using the `mail-auth`
canonicalization implementation. Keys are PKCS#8 DER files projected from a
secret manager, with restrictive file permissions and an explicit active
selector per From domain. The keygen utility creates keys and emits the DNS TXT
record. Tenant-managed lifecycle APIs, protected KMS integration, overlap
automation, audit records, and live provider interoperability remain
outstanding. RSA signing is not enabled in the current profile.

