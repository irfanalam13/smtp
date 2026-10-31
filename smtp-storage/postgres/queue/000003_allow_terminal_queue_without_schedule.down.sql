UPDATE mail_queue
SET next_attempt_at = COALESCE(next_attempt_at, NOW())
WHERE next_attempt_at IS NULL;

ALTER TABLE mail_queue ALTER COLUMN next_attempt_at SET NOT NULL;
