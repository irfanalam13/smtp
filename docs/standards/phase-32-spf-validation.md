# Phase 32 — SPF Validation

SPF follows RFC 7208. Evaluation must use the SMTP client IP, envelope
sender/HELO identity, bounded DNS recursion, and explicit qualifier results.
`+`, `-`, `~`, and `?` map to pass, fail, softfail, and neutral.

Inbound enforcement delegates to the `mail-auth` SPF implementation, which
handles all RFC 7208 mechanisms/modifiers, recursion, lookup and void-lookup
limits, and result classes. The older core helper remains a simplified
reference function and is not used for enforcement. DNSSEC status exposure,
per-message cross-protocol lookup accounting, RFC-indexed fixtures, and live
fixture-domain evidence remain outstanding.

