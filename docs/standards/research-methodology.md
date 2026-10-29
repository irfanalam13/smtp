# Research methodology

## Authority order

1. RFC Editor publication and full RFC text.
2. IETF Datatracker document status, history, errata, and working-group records.
3. IANA registries for SMTP extensions, DNS parameters, headers, media types, and service names.
4. Official standards or registry documents referenced by an RFC.
5. Maintainer documentation only for deployed interoperability behavior that the standard does not fully explain.

Search results and secondary articles are discovery aids only. They are not evidence for a normative requirement.

## Verification procedure

For each RFC:

1. Open the RFC Editor or Datatracker record.
2. Record title, publication date, category, status, authors, updates, and obsoletes from the document header and official metadata.
3. Read the abstract, terminology, protocol sections, security considerations, and relevant references.
4. Check whether a later RFC updates, obsoletes, replaces, or clarifies the behavior.
5. Map only the applicable sections to a product profile and component.
6. Record the official URL and verification date.

The inventory uses publication category (`Standards Track`, `BCP`, `Informational`, `Experimental`, or `Historic`) separately from project implementation status (`researched`, `planned`, `implemented`, or `validated`).

## Normative interpretation

RFC 2119 and RFC 8174 define the familiar normative keywords when they appear in the required all-caps form. The research summaries paraphrase requirements and cite sections; they do not reproduce long source passages. Conditional requirements retain their conditions. For example, a server need not advertise SMTPUTF8 unless it implements the associated semantics, but it MUST honor the requirements once it advertises the extension.

## Evidence limits

This phase establishes an implementation plan, not certification. A protocol feature is not compliant because its RFC appears in the inventory. Conformance requires behavior-level tests, negative tests, interoperability tests, and operational evidence. Provider-specific sender policies are tracked separately from Internet standards because they change independently and may be contractual rather than normative.

## Change review

The standards owner reviews RFC Editor updates, IETF Datatracker status changes, IANA registry changes, security advisories, dependency updates, and major mailbox-provider sender requirements at least quarterly and before a protocol release. A changed source triggers an impact review of the inventory, traceability matrix, ADRs, tests, and customer compatibility policy.
