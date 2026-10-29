# Service boundaries

## Selected deployable boundaries

| Component       | Owns                                                                                                                 | Does not own                                            | Interfaces                                       | Initial deployment             |
| --------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------ | ------------------------------ |
| API edge        | TLS termination, authentication middleware, request limits, routing                                                  | Tenant data or SMTP delivery                            | HTTPS REST, OpenAPI                              | With control plane             |
| Control plane   | tenants, users/service accounts, domains, identities, credentials, policy, quotas, billing references, admin actions | Raw message delivery and SMTP sessions                  | REST, control events                             | Go process                     |
| SMTP submission | SMTP AUTH/submission session, parser limits, sender/recipient policy, acceptance handoff                             | Tenant lifecycle, delivery retry state                  | SMTP, internal acceptance contract               | Rust process, isolated pool    |
| SMTP transport  | remote SMTP sessions, MX/DNS, TLS, response classification                                                           | Authorization and queue ownership                       | SMTP, delivery result                            | Rust worker pool               |
| Queue/scheduler | durable message metadata, leases, due scheduling, retry state                                                        | Remote protocol details, customer webhook calls         | PostgreSQL transaction, internal worker contract | Rust worker process            |
| Event/webhook   | event publication, customer webhook retries and signing                                                              | Source-of-truth business state                          | Durable events, HTTPS                            | Initially one modular consumer |
| Bounce/abuse    | outcome classification, suppression, risk signals, suspension requests                                               | Raw SMTP transport and tenant CRUD                      | Events, policy commands                          | Modular control/data component |
| Inbound/mailbox | mailbox provisioning contract, recipient routing, raw inbound persistence                                            | Outbound queue and webmail UX                           | SMTP, mailbox API                                | Opt-in later process           |
| Analytics       | derived aggregates and reports                                                                                       | Authoritative delivery or tenant state                  | Event consumers, read API                        | Async consumer                 |
| Dashboard       | browser UX                                                                                                           | Authorization decisions and durable writes outside APIs | HTTPS API                                        | Next.js                        |

The queue and delivery worker may share a binary initially because they have a tight lease/result contract. They should split when delivery CPU/network saturation, independent release cadence, or incident isolation requires it. The API edge may be a reverse proxy rather than a bespoke service.

## Ownership and access

Each component has one authoritative write owner. Other components consume APIs/events or read explicitly approved projections. Shared writable business tables are prohibited. PostgreSQL roles and migrations enforce this boundary as the schema evolves.

## Synchronous dependencies

Allowed on acceptance path:

- submission → local policy snapshot/cache;
- submission → PostgreSQL acceptance transaction;
- queue worker → PostgreSQL lease transaction;
- delivery worker → DNS and remote SMTP.

Discouraged or prohibited on acceptance path:

- submission → dashboard, analytics, billing, webhook, or control-plane API;
- delivery → live policy API for every recipient;
- any service → unbounded retry chain.

## Asynchronous interfaces

Control changes, accepted-message notifications, delivery outcomes, bounces, suppression updates, audit records, and usage facts use a durable event envelope. Consumers store offsets/idempotency keys and can replay from retained events.

## Independent deployment conditions

Split a module into a separately deployed service only when at least one is true:

1. It needs materially different scaling or resource limits.
2. It needs a distinct trust boundary or incident blast radius.
3. It has an independent release cadence and stable contract.
4. It can own data without distributed transactions.

Do not split a component only to create a network wrapper around a database or third-party API.

## Alternative decomposition

A modular monolith containing API, policy, queue, and events is suitable for local development and early testing. It is not selected as the production target because a slow control-plane query or API memory spike could consume the same resources as SMTP acceptance. A 15+ service decomposition is also rejected until measured workload and team ownership justify it.
