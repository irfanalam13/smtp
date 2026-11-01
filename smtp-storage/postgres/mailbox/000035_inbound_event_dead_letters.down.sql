DROP INDEX IF EXISTS mailbox_event_outbox_dead_letter_idx;

ALTER TABLE mailbox_event_outbox
    DROP COLUMN IF EXISTS dead_lettered_at;
