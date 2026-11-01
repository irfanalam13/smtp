ALTER TABLE mailbox_event_outbox
    ADD COLUMN dead_lettered_at TIMESTAMPTZ;

CREATE INDEX mailbox_event_outbox_dead_letter_idx
    ON mailbox_event_outbox (dead_lettered_at, created_at)
    WHERE dead_lettered_at IS NOT NULL;
