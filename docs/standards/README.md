# Phase 02 — Standards & RFC Research

Status: **Researched / implementation planning**  
Verification date: **2026-10-28**  
Scope: standards relevant to the Phase 01 transactional email delivery product, with explicit inbound, authentication, and hosted-mailbox extension profiles.

This directory is a standards reference, not a claim that the repository is RFC-compliant. The current repository contains protocol prototypes and architecture documentation; it does not yet implement the standards described here.

## Start here

1. [Research methodology](research-methodology.md)
2. [Standards scope](standards-scope.md)
3. [RFC inventory and relationships](standards-inventory.md)
4. [Normative requirement traceability](requirement-traceability.md)
5. [Protocol feature map](protocol-feature-map.md)
6. [Conformance testing strategy](conformance-testing-strategy.md)
7. [Phase 01 architecture mapping](architecture-mapping.md)
8. [Governance and deprecation](standards-governance.md)
9. [Unresolved questions](unresolved-questions.md)
10. [Phase 11 email protocol engine foundation](phase-11-email-protocol-engine.md)
11. [Phase 12 SMTP core engine](phase-12-smtp-core-engine.md)
12. [Phase 13 SMTP session state machine](phase-13-smtp-session-state-machine.md)
13. [Phase 14 SMTP security](phase-14-smtp-security.md)
14. [Phase 15 ESMTP extensions](phase-15-esmtp-extensions.md)
15. [Phase 16 mail queue engine](phase-16-mail-queue-engine.md)
16. [Phase 17 outbound delivery engine](phase-17-outbound-delivery-engine.md)
17. [Phase 18 inbound mail processing](phase-18-inbound-mail-processing.md)
18. [Phase 19 MIME engine](phase-19-mime-engine.md)
19. [Phase 20 message parser](phase-20-message-parser.md)
20. [Phase 21 IMAP foundation](phase-21-imap-foundation.md)
21. [Phase 22 IMAP core commands](phase-22-imap-core-commands.md)
22. [Phase 23 advanced IMAP features](phase-23-imap-advanced-features.md)
23. [Phase 24 POP3 server](phase-24-pop3-server.md)
24. [Phase 25 mailbox storage engine](phase-25-mailbox-storage-engine.md)
25. [Phase 26 search and indexing](phase-26-search-and-indexing.md)
26. [Phase 27 threading and conversations](phase-27-threading-and-conversations.md)
27. [Phase 28 attachment engine](phase-28-attachment-engine.md)
28. [Phase 29 mail synchronization](phase-29-mail-synchronization.md)
29. [Phase 30 offline mail support](phase-30-offline-mail.md)
30. [Phase 31 DNS integration](phase-31-dns-integration.md)
31. [Phase 32 SPF validation](phase-32-spf-validation.md)
32. [Phase 33 DKIM signing](phase-33-dkim-signing.md)
33. [Phase 34 DKIM verification](phase-34-dkim-verification.md)
34. [Phase 35 DMARC engine](phase-35-dmarc-engine.md)
35. [Phase 36 ARC support](phase-36-arc-support.md)
36. [Phase 37 BIMI support](phase-37-bimi-support.md)
37. [Phase 38 email authentication pipeline](phase-38-email-authentication-pipeline.md)
38. [Phase 39 bounce and delivery status](phase-39-bounce-and-delivery-status.md)
39. [Phase 40 standards compliance and interoperability](phase-40-standards-compliance-and-interoperability.md)
40. [Phase 41 mailbox platform architecture](phase-41-mailbox-architecture.md)
41. [Phase 42 user mailboxes](phase-42-user-mailboxes.md)
42. [Phase 43 folder management](phase-43-folder-management.md)
43. [Phase 44 message storage engine](phase-44-message-storage-engine.md)
44. [Phase 45 message metadata](phase-45-message-metadata.md)
45. [Phase 46 attachment storage](phase-46-attachment-storage.md)
46. [Phase 47 MIME parsing](phase-47-mime-parsing.md)
47. [Phase 48 mail indexing](phase-48-mail-indexing.md)
48. [Phase 49 full-text search](phase-49-full-text-search.md)
49. [Phase 50 search optimization](phase-50-search-optimization.md)
50. [Phase 51 conversation threading](phase-51-conversation-threading.md)
51. [Phase 52 mail filters](phase-52-mail-filters.md)
52. [Phase 53 rule engine](phase-53-rule-engine.md)
53. [Phase 54 labels and categories](phase-54-labels-and-categories.md)
54. [Phase 55 tags and custom properties](phase-55-tags-and-custom-properties.md)
55. [Phase 56 spam folder management](phase-56-spam-folder-management.md)
56. [Phase 57 trash management](phase-57-trash-management.md)
57. [Phase 58 archive system](phase-58-archive-system.md)
58. [Phase 59 draft management](phase-59-draft-management.md)
59. [Phase 60 sent mail management](phase-60-sent-mail-management.md)
60. [Phase 61 favorites and stars](phase-61-favorites-and-stars.md)
61. [Phase 62 read-status synchronization](phase-62-read-status-synchronization.md)
62. [Phase 63 mailbox quotas](phase-63-mailbox-quotas.md)
63. [Phase 64 retention policies](phase-64-retention-policies.md)
64. [Phase 65 mail export and import](phase-65-mail-export-import.md)
65. [Phase 66 backup and recovery](phase-66-backup-and-recovery.md)
66. [Phase 67 synchronization engine](phase-67-synchronization-engine.md)
67. [Phase 68 offline mail](phase-68-offline-mail.md)
68. [Phase 69 storage optimization](phase-69-storage-optimization.md)
69. [Phase 70 mailbox platform integration](phase-70-mailbox-platform-integration.md)

The Markdown tables are the maintained source of truth for this phase. A
machine-readable export can be added in a later phase when controlled values,
schema ownership, and automated update tooling are agreed; no partial export is
presented as authoritative.

## Requirement classifications

- **MUST / MUST NOT** — normative protocol behavior required when the selected profile and applicability conditions hold.
- **SHOULD / SHOULD NOT** — normative recommendation that needs a documented reason to deviate.
- **MAY** — optional behavior; support does not follow merely from reading the RFC.
- **Required for initial release** — P0 delivery capability.
- **Required before production launch** — must be completed before public service, even if not in the current prototype.
- **Planned future capability** — P1/P2 extension.
- **Optional interoperability extension** — useful where supported, but not a baseline commitment.
- **Out of scope** — deliberately excluded with a reconsideration trigger.

RFC terms are interpreted in the context stated by each RFC. A recommendation in one specification is not silently promoted to a universal platform requirement.

## Research status

The inventory covers the core SMTP, message/MIME, DNS, authentication, TLS, submission, internationalization, DSN, abuse, privacy, mailbox, and API standards needed to plan implementation. RFC publication metadata was checked against RFC Editor text and official IETF/IANA pages on the verification date. Individual implementation status remains **not implemented** unless the repository explicitly proves otherwise.

## Official sources

- [RFC Editor](https://www.rfc-editor.org/)
- [IETF Datatracker](https://datatracker.ietf.org/)
- [IANA SMTP parameters](https://www.iana.org/assignments/smtp/)
- [IANA DNS parameters](https://www.iana.org/assignments/dns-parameters/)
- [IANA service names and port numbers](https://www.iana.org/assignments/service-names-port-numbers/)
- [IANA media types](https://www.iana.org/assignments/media-types/)

## Remaining implementation

This phase documents normative requirements and research; it does not make the
prototype RFC-compliant by itself. The remaining protocol implementation,
conformance, and interoperability work, including why it is deferred and when
to apply it, is tracked in the [phase implementation status](../phase-status.md)
under Phase 02.
