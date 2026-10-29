# Phase 25 — Mailbox Storage Engine

Status: **Foundation schema implemented; retrieval engine not complete**

The storage contract separates mailbox folders, stable UIDs, raw message
bytes, flags, keywords, internal dates, and deletion timestamps. Indexes cover
folder/UID and date retrieval. Production completion requires transactional
UID allocation, concurrent mutation rules, full-text/header indexes, quota
accounting, compaction/retention, blob tiering, and backup/restore evidence.
