# Phase 117 — Distributed Tracing

Status: **No end-to-end trace exporter or collector**

## Goal and scope

Trace request and message-processing paths across API, SMTP ingress, durable
queue, workers, storage, and external dependencies. Traces are diagnostic
telemetry, not durable audit evidence and not a substitute for message
delivery state.

## Repository baseline

The repository has request/correlation identifiers and local Rust tracing;
the Go API logs a request ID. There is no end-to-end W3C Trace Context
propagation, OpenTelemetry SDK/exporter, collector, or cross-service trace
backend.

## Recommended design

Adopt W3C Trace Context at trusted boundaries, validate and bound incoming
context, and create spans around meaningful operations and dependency calls.
Propagate context through durable work metadata only under a reviewed schema
and privacy policy. Do not propagate arbitrary baggage. Never add addresses,
message contents, access tokens, API keys, or high-cardinality tenant/object
identifiers as span attributes. Sampling must preserve error/slow-path
diagnostic value while bounding cost.

Trace identifiers must not be accepted as authorization credentials. Trace
sampling and export failures must not change SMTP acceptance, mailbox
consistency, or API success semantics.

## Remaining work and why

Select telemetry collector and backend, document trust boundaries and sampling,
add instrumentation to the independent Rust and Go services, and verify trace
continuity through durable asynchronous handoff. This remains open because the
collector, second deployed service boundary, backend, and privacy review are
not present.

## Acceptance criteria

- A test trace can be followed across API/SMTP ingress, queue, and worker
  without adding secrets or message content to attributes.
- Invalid/untrusted trace headers cannot alter authorization or trust decisions.
- Asynchronous trace linkage survives enqueue/retry while business state stays
  authoritative in the queue.
- Sampling/export outages do not block service operations.
- Trace access, retention, and sampling costs have documented owners.

## Research references

- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [OpenTelemetry distributed tracing](https://opentelemetry.io/docs/concepts/signals/traces/)
- [Phase 82 Event Bus and Messaging](phase-82-event-bus-messaging.md)
