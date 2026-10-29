# Phase 32 — SPF Validation

SPF follows RFC 7208. Evaluation must use the SMTP client IP, envelope
sender/HELO identity, bounded DNS recursion, and explicit qualifier results.
`+`, `-`, `~`, and `?` map to pass, fail, softfail, and neutral.

The core foundation parses direct `ip4`, `ip6`, and `all` mechanisms and returns
explicit results. Include, redirect, DNS lookup limits, macro expansion,
void-lookups, and DNSSEC-aware retrieval remain adapter work.

