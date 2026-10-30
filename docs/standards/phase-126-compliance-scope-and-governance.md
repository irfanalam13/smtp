# Phase 126 — Compliance Scope and Governance

Status: **Security guidance exists; applicability assessments and formal evidence program pending**

## Goal and scope

Determine which legal, contractual, and assurance requirements apply to the
actual service and establish accountable governance and evidence collection.
GDPR, HIPAA, SOC 2, ISO/IEC 27001, and PCI DSS are different legal/assurance
frameworks; they are not interchangeable software feature sets.

## Repository baseline

The repository has security architecture and audit guidance but no completed
data inventory, jurisdictional legal assessment, risk analysis, ISMS, SOC
report, HIPAA program, or PCI DSS scope validation. SMTP service capabilities
alone do not establish that the platform is compliant or certified.

## Recommended design

Maintain a data-flow inventory, data classifications, processing purposes,
data subjects, locations, subprocessors, retention, and access owners. Have
qualified counsel and compliance professionals determine applicability,
controller/processor obligations, covered-entity/business-associate status,
payment-card scope, contractual commitments, and evidence requirements.

Map controls to a selected framework and maintain owners, policies, evidence,
exceptions, risk acceptance, remediation, and review dates. Do not collect
payment card data unless product scope and PCI controls are explicitly
approved; use a payment provider/tokenization boundary where appropriate.

## Remaining work and why

Complete organizational applicability and risk assessments, establish
governance owners, and plan independent assessment where required. This
remains open because legal applicability and assurance require organizational
facts and independent review, not code alone.

## Acceptance criteria

- Data inventory, processing map, regions, subprocessors, and retention are
  reviewed and owned.
- Each claimed framework has a documented scope and independent legal/audit
  validation where applicable.
- Policies, control owners, evidence, exceptions, and remediation are tracked.
- No certification or compliance claim is made from implementation checklists
  alone.
- Payment data is excluded or handled in a separately approved and assessed
  environment.

## Research references

- [GDPR — Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj)
- [HHS HIPAA Security Rule resources](https://www.hhs.gov/hipaa/for-professionals/security/index.html)
- [AICPA SOC Suite](https://www.aicpa-cima.com/resources/landing/system-and-organization-controls-soc-suite-of-services)
- [ISO/IEC 27001:2022](https://www.iso.org/standard/27001)
- [PCI Security Standards Council — PCI DSS](https://www.pcisecuritystandards.org/standards/pci-dss/)
