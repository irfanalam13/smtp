# Phase 91 — White-Label Platform

Status: **Deployment-level dashboard name configurable; tenant branding, custom domains, and identity pages pending**

## Repository baseline and initial implementation

The Next.js dashboard currently has a fixed product label and generated
metadata. It can now take a public deployment-level `NEXT_PUBLIC_BRAND_NAME`
value; when unset or blank it displays `SMTP Platform`. The value is rendered
as text and used for page metadata. This is a build/deployment brand, not a
tenant-controlled setting or a complete white-label product.

For the current Ubuntu-laptop scope, this deployment-level setting is enough
to preview branding at localhost. Custom application domains, tenant branding,
and public TLS are deferred; they are not prerequisites for running locally.

The API has tenant and mail-domain tables, but there is no verified
application-hostname mapping, tenant branding record, customer logo store,
tenant-specific login flow, or theme API. Outbound email templates are also
not implemented.

## Recommended white-label model

Keep platform deployment branding separate from tenant branding. Tenant
branding must resolve only after a verified hostname-to-tenant mapping and
authenticated tenant context; an arbitrary `Host`, forwarded-host, query, or
client-supplied tenant value must never select authorization scope. Custom
application domains require domain ownership challenges, certificate
provisioning/renewal, host allowlisting, safe proxy behavior, tenant-conflict
handling, and domain-removal cleanup. Sending domains are not automatically
login/application domains.

Store validated theme tokens and asset object references, not executable CSS
or arbitrary HTML. Upload logos through a bounded, scanned, tenant-owned asset
pipeline; serve them from a controlled asset origin with safe content types
and cache policy. Do not fetch tenant-provided remote logo URLs from the
server. Escape tenant text in HTML and email templates. Branding may alter
presentation but never authentication, authorization, security notices,
terms, or platform attribution required by policy.

## Remaining work and why

Define tenant branding fields and defaults, ownership/admin permissions,
custom application-domain verification and TLS lifecycle, logo asset
processing, email branding/templates, theme accessibility, account discovery,
login/logout behavior, platform attribution, preview, audit, and rollback.
Implement tenant resolution only after Phases 72 and 74 provide tested
tenant isolation and production identity.

This remains open because the platform lacks durable tenant configuration,
production identity, object storage, and application-domain verification.
Resolving tenant identity from an unverified hostname could route users to a
spoofed login or leak cross-tenant information. The deployment-level brand
name is safe groundwork but does not satisfy tenant-specific white-labeling.

## Acceptance criteria

- Host-to-tenant resolution uses verified, unique domains and cannot be
  overridden by request parameters or untrusted proxy headers.
- Domain ownership, certificate issue/renew/revoke, transfer, collision, and
  deletion are auditable and tested.
- Theme values are schema-limited, accessible, escaped, and cannot inject
  scripts/styles or suppress security controls.
- Logo assets are scanned, size-bounded, isolated by tenant, and served with
  safe content headers.
- Login, security notifications, email templates, and API responses never
  confuse tenant branding with identity or authorization.

## Research references

- [Phase 71 SaaS Architecture](phase-71-saas-architecture.md)
- [Phase 72 Multi-Tenant Database](phase-72-multi-tenant-database.md)
- [Phase 74 Authentication and Identity](phase-74-authentication-identity.md)
- [OWASP Cross Site Scripting Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html)
- [W3C Web Content Accessibility Guidelines 2.2](https://www.w3.org/TR/WCAG22/)
