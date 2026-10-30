# Phase 101 — Zero Trust and Service Identity

Status: **Partial application-level tenant checks; workload identity and verified service-to-service trust pending**

## Repository baseline and initial code slice

The control plane has typed principals, scoped API keys, tenant/resource
matching, default-deny authorization foundations, and an optional Keycloak
OIDC resource-server verifier. Network location is not intended to establish
trust. No OIDC deployment is configured, and workload identity, service mesh,
and service-to-service credential lifecycle remain absent.

The API now rejects `CONTROL_PLANE_DEV_MODE=true` whenever
`APP_ENVIRONMENT` is unset or is not `development`/`dev`, including staging
and unknown labels. This is a startup guardrail, not Zero Trust completion.

## Recommended design

Authenticate each human, workload, and service independently; authorize each
request against identity, tenant, resource, action, and current policy.
Use short-lived workload credentials from the selected cloud/runtime identity
provider or a reviewed mTLS identity system. Never treat VPC membership,
source IP, cluster membership, or a caller-provided tenant ID as identity.
Separate customer, operator, and workload identities and rotate/revoke
credentials centrally.

## Remaining work and why

Configure the selected Keycloak issuer/audience and account lifecycle; select
EKS workload identity (IRSA or EKS Pod Identity); propagate tenant-bound
identity safely across service calls; define token lifetime, revocation, key
rotation, and incident response; and test spoofing, replay, confused-deputy,
and cross-tenant cases. This remains open because no production IdP or EKS
deployment is configured, and current service boundaries are not yet deployed
as independently authenticated workloads.

## Acceptance criteria

- Every service request has verifiable workload/user identity and an explicit
  audience; network location alone grants no authority.
- Tenant and resource authorization is enforced at each data-access boundary.
- Credentials are short-lived, revocable, rotated, and absent from logs.
- Production, staging, and test cannot enable development identity.
- Negative tests cover forged identity, replay, confused deputy, and
  cross-tenant access.

## Research references

- [NIST SP 800-207 Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final)
- [Phase 07 security architecture](../security/security-architecture.md)
- [ADR 0001 security foundation boundary](../architecture/adr/phase-07/0001-security-foundation-boundary.md)
