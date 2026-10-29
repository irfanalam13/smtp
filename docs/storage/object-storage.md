# Object storage policy

Object storage is not currently integrated. Existing queue and inbound
prototypes intentionally store bounded payloads in PostgreSQL and do not have
an approved S3-compatible provider or local emulator.

Future object-backed content must use private buckets, provider-supported
checksums, bounded streaming or multipart uploads, generated keys independent
of user filenames, and explicit metadata ownership in PostgreSQL. Metadata and
object operations cannot share a PostgreSQL ACID transaction. Upload and
metadata failure windows require idempotent operations, reconciliation, and
orphan handling.

No public bucket, signed-URL logger, cloud credential, or placeholder adapter is
introduced in Phase 06.
