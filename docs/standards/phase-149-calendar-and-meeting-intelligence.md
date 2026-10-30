# Phase 149 — Calendar and Meeting Intelligence

Status: **No meeting extraction, calendar integration, or scheduling API implemented**

## Repository baseline

The platform parses email MIME but does not implement calendar APIs or iCalendar
processing. Extracting a meeting suggestion must not create invitations or
modify a user's calendar.

## Recommended design

Parse explicit `text/calendar`/iCalendar parts with a standards-conformant
parser and bounded input, then distinguish verified structured fields from
model-inferred suggestions. Preserve timezone identifiers, recurrence,
organizer/attendee semantics, cancellations, and updates. Validate
authorization and provider scopes for calendar access. Require user review
before creating, accepting, declining, or modifying events.

## Remaining work and why

Choose calendar providers and delegated scopes, implement iCalendar parsing,
timezone and recurrence handling, consent and conflict policy, and safe
authorization. This remains open because no calendar product, provider,
parser, or permission model exists.

## Acceptance criteria

- Structured calendar content follows RFC 5545 and is bounded against
  oversized/recursive input.
- Timezone, recurrence, organizer, cancellation, and update semantics are
  covered by conformance fixtures.
- External calendar access uses least-privilege delegated authorization and
  secure token handling.
- Model-extracted times are labeled as suggestions and never silently create
  or change events.
- Provider failures or ambiguous timezones require user clarification.

## Research references

- [RFC 5545 — iCalendar](https://www.rfc-editor.org/rfc/rfc5545)
- [RFC 5322 — Internet Message Format](https://www.rfc-editor.org/rfc/rfc5322)
- [Phase 47 MIME Parsing](phase-47-mime-parsing.md)
