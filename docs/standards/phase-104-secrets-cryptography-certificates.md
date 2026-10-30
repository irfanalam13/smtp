# Phase 104 — Secrets, Cryptography, and Certificate Lifecycle

Status: **AWS KMS and Secrets Manager selected; integration, encryption-at-rest, certificate automation, and HSM evidence pending**

## Repository baseline

The project uses Rustls/system roots for outbound SMTP STARTTLS, operating
system randomness for API-key generation, and high-entropy API-key verifier
hashing. Production TLS termination, private-key storage, key encryption,
managed secret delivery, KMS/HSM, certificate issuance/renewal, and
cryptographic key lifecycle are not deployed.

## Recommended design

Use AWS Secrets Manager for runtime secrets and AWS KMS for managed key
operations; use IAM Roles for Service Accounts (IRSA) or EKS Pod Identity,
not long-lived AWS access keys in pods. Separate data-encryption, signing,
transport, API credential, and database-secret key purposes. Use envelope
encryption for stored message/attachment data only after tenant/key ownership
and backup/restore semantics are designed. Keep production TLS at an approved
edge or listener with authenticated upstream connections. Automate
certificate issuance/renewal/revocation and alert on expiry. HSM support is a
provider integration requirement, not a local software flag.

## Remaining work and why

Implement least-privilege AWS IAM roles and KMS/Secrets Manager access; define
key hierarchy, data classes, residency, rotation and recovery policy; design
encryption key versioning and migration; automate certificates; test service
outage, rotation, revocation, and restore. This remains open because IAM
ownership, production AWS accounts, object storage, message retention, and key
custodianship are not configured.

## Acceptance criteria

- No application-owned cryptographic implementation or hardcoded production
  secret/key is introduced.
- Key purpose, owner, version, access, rotation, revocation, recovery, and
  audit are documented and enforced.
- TLS endpoint identity and upstream trust are validated; expiry and renewal
  failures alert before outage.
- Encryption/decryption survives tested key rotation, backup restore, and
  authorized tenant deletion workflows.
- HSM claims are made only after provider integration and operational testing.

## Research references

- [NIST SP 800-57 Part 1 key management recommendations](https://csrc.nist.gov/pubs/sp/800/57/pt1/r5/final)
- [Phase 07 cryptography and TLS](../security/cryptography-and-tls.md)
- [Secrets and key management policy](../security/secrets-and-key-management.md)
- [Amazon EKS security best practices](https://docs.aws.amazon.com/eks/latest/best-practices/security.html)
