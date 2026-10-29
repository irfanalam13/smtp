# Durable mail queue

Milestone 2 stores accepted test messages in PostgreSQL before SMTP returns
success. `mail_messages` contains the envelope sender and bounded raw message
bytes; `mail_recipients` contains the recipient envelope; `mail_queue` contains
the processing state and lease; `mail_attempts` is append-oriented history; and
`mail_idempotency` maps a scoped key and SHA-256 request fingerprint to the
message.

## States and transitions

```text
queued -> processing -> simulated_succeeded
                    -> retry_scheduled -> processing
                    -> terminal_failed
processing (expired lease) -> retry_scheduled
```

Claims use `FOR UPDATE SKIP LOCKED` and commit a unique lease token and bounded
expiry before processing. Completion requires that token, so a stale worker
cannot modify a reclaimed job. A successful test processor means only that the
simulation succeeded; it is not remote delivery.

Enqueue inserts the message, recipients, queue row, and idempotency record in
one transaction. Duplicate keys with the same fingerprint return the original
message ID. A changed request using the same scoped key is a conflict. The
current implementation retains rows indefinitely for test inspection; a
production retention policy and object storage for large bodies are future
work.

Retries use bounded exponential backoff with bounded random jitter. Temporary
and internal failures retry until the configured maximum; permanent, cancelled,
and exhausted failures become terminal. The guarantee is at-least-once
processing attempts, not exactly-once delivery.
