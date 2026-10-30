# Observability

The current repository uses structured `tracing` logs in Rust and a small,
dependency-free observability layer in the Go API. The application remains
independent of a hosted telemetry vendor.

## Implemented

- Rust services initialize compact development/test logs and JSON
  production-oriented logs through `shared-types::logging`.
- Go API requests emit structured completion logs with method, bounded route,
  status, duration, and request ID. Request bodies, headers, tokens, and
  message content are not logged.
- The Go API exports bounded Prometheus text metrics at `/metrics`. This
  endpoint is intended for a private local or deployment network, not public
  customer access.
- The Go API metrics include a process-local HTTP request-duration histogram
  in seconds and an in-flight request gauge, in addition to aggregate request,
  failure, and response-class counters.
- `/health/live` reports process liveness without dependency calls.
- `/health/ready` reports whether the current development API is initialized.
- Request metrics use aggregate status classes and do not label by request ID,
  tenant, email address, message ID, or path parameter.

## Metrics

Current metric names:

- `smtp_api_http_requests_total`
- `smtp_api_http_failures_total`
- `smtp_api_http_request_duration_microseconds_total`
- `smtp_api_http_active_requests`
- `smtp_api_http_request_duration_seconds` (classic histogram)
- `smtp_api_http_responses_total{status_class="1xx|2xx|3xx|4xx|5xx"}`

These are process-local metrics. They are useful for local development and
single-process scraping, but they are not distributed or durable monitoring.
The histogram uses fixed latency buckets and low-cardinality data; aggregate
buckets across replicas in the monitoring backend to calculate fleet
percentiles. Do not label metrics with tenant IDs, message IDs, email
addresses, request IDs, or raw paths.

## Tracing and export

The repository has distinct request, correlation, and trace identifiers in the
shared Rust context model, and the API preserves bounded request IDs. A full
OpenTelemetry exporter, W3C trace-context propagation, collector, central log
backend, and distributed aggregation are deferred because no collector or
second deployed service boundary is currently available.

## Local validation

```sh
(cd smtp-api && go test ./...)
curl http://localhost:8080/health/live
curl http://localhost:8080/health/ready
curl http://localhost:8080/metrics
```

Do not expose `/metrics` or diagnostic dashboards publicly. Restrict scraping
to a private operations network and apply the deployment's authentication and
authorization controls. See the [Phase 111–120 designs](../standards/phase-111-scalability-architecture.md)
for the remaining scalability and observability work.
