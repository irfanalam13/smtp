# Phase 167 — CDN Integration

Status: **No CDN distribution or attachment delivery integration exists**

## Recommended design

Use a CDN initially for versioned public static assets. Private attachments
require authorization-bound, short-lived delivery tokens or an authenticated
origin; avoid public cache keys or reusable signed URLs. Set content security,
disposition, MIME sniffing, cache-control, revocation, and deletion behavior.
Never cache mailbox/API responses across users. Ensure cache invalidation
honors legal deletion and tenant residency.

## Remaining work and why

Choose asset and private-download flows, origin protection, cache policy,
token lifetime, residency, and purge semantics. This remains open because
object storage, attachment APIs, and global regions are not deployed.

## Acceptance criteria

- Public assets and private mail content have separate distributions/policies.
- Unauthorized, expired, revoked, or cross-tenant download tests fail closed.
- Sensitive downloads use safe headers and cannot be cached publicly.
- Deletion/retention workflows invalidate origin and edge copies.
- CDN logs and cache keys exclude unnecessary personal/message identifiers.

## Research references

- [Amazon CloudFront introduction](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Introduction.html)
- [Phase 46 Attachment Storage](phase-46-attachment-storage.md)
- [Phase 127 Retention and eDiscovery](phase-127-retention-legal-hold-and-ediscovery.md)
