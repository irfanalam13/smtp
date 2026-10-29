# Unresolved standards questions

1. Which SMTP AUTH mechanisms will be permitted for initial public submission, and is OAuth SASL required?
2. Is SMTPUTF8 needed for the first customer segment, or can the launch profile reject messages requiring it without lossy downgrade?
3. Which DKIM algorithm/key-size policy and KMS deployment are approved?
4. Which DMARC aggregate/failure reporting data may be retained and exposed under customer privacy requirements?
5. Will the first production region support DANE validation, MTA-STS retrieval, TLS reporting, or only authenticated STARTTLS plus telemetry?
6. What provider-specific bulk-sender requirements and unsubscribe obligations apply to planned customer segments?
7. Is inbound mail a launch requirement, and if so, which recipient routing and anti-abuse profile is required?
8. If hosted mailboxes are funded, is IMAP-only sufficient or is POP3 required for a defined customer segment?
9. Which IDNA version and Unicode normalization policy will be used for internationalized domains and addresses?
10. Which external interoperability peers, fuzzing tools, and test fixtures can be approved under their current licenses?

These questions are inputs to Phases 03, 07, 08, 09, and 10. They do not block documenting the P0 standards baseline, but they block claims of production conformance for the affected capabilities.
