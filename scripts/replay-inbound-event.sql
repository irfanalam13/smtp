-- Run against an isolated or explicitly selected operational database.
-- Supply the event UUID as the psql variable: psql ... -v event_id='...'
UPDATE mailbox_event_outbox
SET dead_lettered_at = NULL,
    attempts = 0,
    available_at = NOW(),
    locked_by = NULL,
    locked_until = NULL,
    last_error = NULL
WHERE id = :'event_id'
  AND published_at IS NULL
  AND dead_lettered_at IS NOT NULL;
