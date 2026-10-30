DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM inbound_messages AS message
        LEFT JOIN mailboxes AS mailbox
          ON mailbox.id = message.mailbox_id
         AND mailbox.tenant_id = message.tenant_id
        WHERE mailbox.id IS NULL
    ) THEN
        RAISE EXCEPTION
            'cannot enforce inbound mailbox tenant ownership: inconsistent rows exist';
    END IF;
END
$$;

ALTER TABLE inbound_messages
    ADD CONSTRAINT inbound_messages_mailbox_tenant_fk
    FOREIGN KEY (mailbox_id, tenant_id)
    REFERENCES mailboxes (id, tenant_id)
    ON DELETE RESTRICT;
