# Phase 17 — Outbound Delivery Engine

Status: **Bounded SMTP delivery foundation exists; production delivery profile
is not complete**

The delivery component already performs bounded MX resolution, destination
safety checks, SMTP greeting/EHLO, STARTTLS policy, envelope delivery, DATA
dot-stuffing, reply parsing, and candidate failover.

The next implementation boundary is to connect it to the real queue processor
and preserve recipient-level outcomes. Connection pooling must be keyed by
destination and TLS policy, with idle/maximum lifetime and failure eviction.
Retry decisions belong to the queue, while SMTP response classification belongs
to delivery. No delivery result may claim inbox placement.
