ALTER TABLE mail_recipients
    ADD COLUMN recipient_kind TEXT NOT NULL DEFAULT 'to'
        CHECK (recipient_kind IN ('to', 'cc', 'bcc'));
