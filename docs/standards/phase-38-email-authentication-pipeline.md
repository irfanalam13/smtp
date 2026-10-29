# Phase 38 — Email Authentication Pipeline

## Standards baseline

The evaluation path combines SPF (RFC 7208), DKIM (RFC 6376 and updates),
DMARC (current DMARC standards and legacy RFC 7489 interoperability), and
optional ARC (Experimental RFC 8617). BIMI is a downstream brand-display
eligibility signal, not authentication. Each mechanism retains its own result
and error class; temporary DNS failures must not be collapsed into `fail`.

## Implemented foundation

The core produces a structured report containing SPF, DKIM, DMARC, ARC, and
BIMI results. Disposition is derived from DMARC and does not treat ARC as an
automatic override or BIMI as proof of message authenticity.

## Production requirements

- Feed outcomes from real DNS, DKIM crypto, policy discovery, and trusted ARC
  evaluation adapters.
- Preserve `none`, `pass`, `fail`, `temperror`, and `permerror` semantics.
- Apply tenant/domain policy and traceable local ARC trust decisions.
- Integrate with inbound quarantine/reject/routing and structured logging.
- Add privacy-aware authentication-result headers and report generation.

