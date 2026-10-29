# Phase 10 performance baseline

## API unauthenticated boundary

| Field | Value |
|---|---|
| Component | Go control-plane API |
| Endpoint | `GET /v1/domains` |
| Environment | Local Linux workstation, Go server, loopback |
| Configuration | `APP_ENVIRONMENT=development`, development identity disabled |
| Workload | 100 sequential requests using `curl` |
| Expected result | HTTP 401 for every request |
| Actual result | 100 HTTP 401 responses, zero HTTP 5xx responses |
| Wall-clock duration | 779.10 ms |
| Approximate sequential wall-clock/request | 7.79 ms |
| Error rate | 0% server errors; authentication rejection is expected |

This is a small smoke baseline, not a capacity claim. It does not measure
authenticated database-backed work, concurrency, tail latency, memory, CPU,
pool saturation, or production network behavior.

## Health endpoint smoke baseline

100 sequential `GET /health/live` requests completed in 736.23 ms
(approximately 7.36 ms wall-clock/request) on the same local setup. Health
requests are intentionally outside the control-plane request metrics
middleware, so this measurement does not appear in
`smtp_api_http_requests_total`.

## Required follow-up

Phase 10 production validation must add controlled concurrency, percentile
latency, resource measurements, authenticated API workloads, database-backed
queries, queue throughput, and dependency-failure scenarios after a target
runtime and representative test environment are selected.
