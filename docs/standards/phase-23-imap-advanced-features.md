# Phase 23 — Advanced IMAP Features

Status: **Not implemented**

Flags and folders are represented in the storage contract. IDLE, quotas, ACLs,
annotations, namespaces, and incremental synchronization require durable
versioning and notification fan-out. These features must be implemented after
core command transactions and mailbox concurrency are proven.
