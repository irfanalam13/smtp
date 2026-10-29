# ADR-0015: Transactional delivery before hosted mailboxes

- Status: Proposed
- Date: 2026-10-28

## Context

The repository has an opt-in inbound prototype, but hosted mailboxes require retention, search, IMAP/POP3, synchronization, abuse controls, and a larger privacy surface.

## Decision

Make transactional outbound delivery, domain verification, durable queueing, delivery outcomes, suppression, and abuse foundations the P0 product. Treat inbound reception as an opt-in P1 extension and hosted mailbox access/search/webmail as future scope.

## Consequences

Launch scope is smaller and operationally tractable. Inbound interfaces must remain compatible, but no current prototype is presented as hosted email production capability.
