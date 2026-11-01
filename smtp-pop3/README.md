# smtp-pop3

POP3 is explicitly out of scope for the first production release. This crate
contains parser and state-machine fixtures used for protocol research and
tests; it does not provide a network listener, authentication, mailbox access,
TLS, or a production maildrop.

No deployment, configuration, dashboard, or release documentation may present
this crate as a usable POP3 service. A future implementation must provide a
real RFC 1939/RFC 8314 service and independent interoperability evidence before
this scope decision changes.
