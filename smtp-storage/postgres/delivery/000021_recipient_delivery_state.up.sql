ALTER TABLE mail_queue
    DROP CONSTRAINT mail_queue_state_check,
    ADD CONSTRAINT mail_queue_state_check
        CHECK (state IN (
            'queued', 'processing', 'retry_scheduled', 'simulated_succeeded',
            'terminal_failed', 'accepted_by_remote', 'partially_accepted',
            'permanently_failed', 'cancelled', 'suppressed',
            'delivery_uncertain', 'delivery_processing'
        ));

CREATE TABLE mail_recipient_deliveries (
    id BIGSERIAL PRIMARY KEY,
    recipient_id BIGINT NOT NULL UNIQUE
        REFERENCES mail_recipients(id) ON DELETE CASCADE,
    message_id UUID NOT NULL REFERENCES mail_messages(id) ON DELETE CASCADE,
    destination_domain TEXT NOT NULL
        CHECK (length(destination_domain) BETWEEN 1 AND 253),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN (
            'pending', 'processing', 'retry_scheduled', 'accepted_by_remote',
            'permanently_failed', 'uncertain', 'suppressed'
        )),
    next_attempt_at TIMESTAMPTZ DEFAULT NOW(),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    lease_owner TEXT CHECK (lease_owner IS NULL OR length(lease_owner) BETWEEN 1 AND 128),
    lease_token UUID,
    lease_expires_at TIMESTAMPTZ,
    last_smtp_code SMALLINT CHECK (last_smtp_code IS NULL OR last_smtp_code BETWEEN 100 AND 599),
    last_diagnostic TEXT CHECK (last_diagnostic IS NULL OR length(last_diagnostic) <= 512),
    last_remote_host TEXT CHECK (last_remote_host IS NULL OR length(last_remote_host) <= 253),
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (
            state = 'processing'
            AND lease_owner IS NOT NULL
            AND lease_token IS NOT NULL
            AND lease_expires_at IS NOT NULL
        )
        OR (
            state <> 'processing'
            AND lease_owner IS NULL
            AND lease_token IS NULL
            AND lease_expires_at IS NULL
        )
    ),
    CHECK (state NOT IN ('pending', 'retry_scheduled') OR next_attempt_at IS NOT NULL)
);

CREATE TABLE mail_recipient_attempts (
    id BIGSERIAL PRIMARY KEY,
    delivery_id BIGINT NOT NULL
        REFERENCES mail_recipient_deliveries(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    outcome TEXT NOT NULL DEFAULT 'in_progress'
        CHECK (outcome IN (
            'in_progress', 'accepted_by_remote', 'temporary_failure',
            'permanent_failure', 'uncertain', 'suppressed'
        )),
    smtp_code SMALLINT CHECK (smtp_code IS NULL OR smtp_code BETWEEN 100 AND 599),
    diagnostic TEXT CHECK (diagnostic IS NULL OR length(diagnostic) <= 512),
    remote_host TEXT CHECK (remote_host IS NULL OR length(remote_host) <= 253),
    UNIQUE (delivery_id, attempt_number),
    CHECK ((outcome = 'in_progress') = (finished_at IS NULL))
);

CREATE INDEX mail_recipient_deliveries_due_idx
    ON mail_recipient_deliveries (next_attempt_at, destination_domain, created_at)
    WHERE state IN ('pending', 'retry_scheduled');
CREATE INDEX mail_recipient_deliveries_lease_idx
    ON mail_recipient_deliveries (lease_expires_at)
    WHERE state = 'processing';
CREATE INDEX mail_recipient_deliveries_message_idx
    ON mail_recipient_deliveries (message_id, state);

CREATE TABLE mail_delivery_domain_leases (
    destination_domain TEXT PRIMARY KEY
        CHECK (length(destination_domain) BETWEEN 1 AND 253),
    lease_owner TEXT NOT NULL CHECK (length(lease_owner) BETWEEN 1 AND 128),
    lease_token UUID NOT NULL,
    lease_expires_at TIMESTAMPTZ NOT NULL
);

CREATE FUNCTION create_mail_recipient_delivery()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    recipient_domain TEXT;
BEGIN
    recipient_domain := lower(rtrim(split_part(NEW.recipient, '@', 2), '.'));
    IF recipient_domain = '' OR length(recipient_domain) > 253 THEN
        RAISE EXCEPTION 'recipient domain is invalid';
    END IF;
    INSERT INTO mail_recipient_deliveries (recipient_id, message_id, destination_domain)
    VALUES (NEW.id, NEW.message_id, recipient_domain);
    RETURN NEW;
END;
$$;

CREATE TRIGGER mail_recipients_create_delivery
AFTER INSERT ON mail_recipients
FOR EACH ROW EXECUTE FUNCTION create_mail_recipient_delivery();

INSERT INTO mail_recipient_deliveries
    (recipient_id, message_id, destination_domain, state, last_diagnostic)
SELECT id, message_id, lower(rtrim(split_part(recipient, '@', 2), '.')),
       'suppressed', 'pre-existing queue item held for explicit migration/replay'
FROM mail_recipients
ON CONFLICT (recipient_id) DO NOTHING;
