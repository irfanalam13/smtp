DROP TRIGGER IF EXISTS mail_recipients_create_delivery ON mail_recipients;
DROP FUNCTION IF EXISTS create_mail_recipient_delivery();
DROP TABLE IF EXISTS mail_delivery_domain_leases;
DROP TABLE IF EXISTS mail_recipient_attempts;
DROP TABLE IF EXISTS mail_recipient_deliveries;

UPDATE mail_queue
SET state = CASE
    WHEN state IN ('accepted_by_remote', 'partially_accepted') THEN 'simulated_succeeded'
    WHEN state IN ('delivery_uncertain', 'delivery_processing', 'permanently_failed', 'cancelled', 'suppressed') THEN 'terminal_failed'
    ELSE state
END;

ALTER TABLE mail_queue
    DROP CONSTRAINT mail_queue_state_check,
    ADD CONSTRAINT mail_queue_state_check
        CHECK (state IN (
            'queued', 'processing', 'retry_scheduled', 'simulated_succeeded',
            'terminal_failed'
        ));
