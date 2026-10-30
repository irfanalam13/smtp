# Phase 165 — Global Load Balancing

Status: **No global accelerator or multi-region load balancer configured**

## Recommended design

Select routing by protocol and traffic semantics. HTTP APIs/web may use
health-aware global acceleration or DNS routing; SMTP, IMAP, and POP3 require
connection-aware L4 handling, long-lived connection behavior, stable
egress/ingress reputation, and protocol-specific drain plans. A global load
balancer does not replicate state or make writes safe across regions.

## Remaining work and why

Choose edge services, static addressing, regional endpoints, health semantics,
connection timeouts, source-IP preservation, DDoS controls, and protocol
testing. This remains open because there are no deployed endpoints or traffic
profiles.

## Acceptance criteria

- Each protocol has an explicit global routing and drain design.
- Health failure removes only unsafe endpoints and recovers without flapping.
- Client IP, TLS, SMTP identity, and egress reputation requirements are met.
- Stateful writes are routed to their authoritative region.
- Regional cutover and return are tested with real protocol clients.

## Research references

- [AWS Global Accelerator](https://docs.aws.amazon.com/global-accelerator/latest/dg/what-is-global-accelerator.html)
- [Route 53 routing policies](https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/routing-policy.html)
- [Phase 107 Distributed Mail Protocol Services](phase-107-distributed-mail-protocol-services.md)
