ALTER TABLE mail_messages
    DROP CONSTRAINT IF EXISTS mail_messages_envelope_sender_check;

ALTER TABLE mail_messages
    ADD CONSTRAINT mail_messages_envelope_sender_check
    CHECK (length(envelope_sender) BETWEEN 0 AND 320);
