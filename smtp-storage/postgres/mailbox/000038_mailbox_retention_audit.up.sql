CREATE FUNCTION audit_mailbox_message_deletion() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    message_tenant_id UUID;
    message_mailbox_id UUID;
BEGIN
    SELECT mailbox.tenant_id, mailbox.id
    INTO message_tenant_id, message_mailbox_id
    FROM mailbox_folders AS folder
    JOIN mailboxes AS mailbox ON mailbox.id = folder.mailbox_id
    WHERE folder.id = OLD.folder_id;

    IF message_tenant_id IS NOT NULL THEN
        INSERT INTO audit_events (
            tenant_id, action, target_type, target_id, metadata
        )
        VALUES (
            message_tenant_id,
            'mailbox.message.retention_deleted',
            'mailbox_message',
            OLD.id::TEXT,
            jsonb_build_object(
                'mailbox_id', message_mailbox_id,
                'deleted_at', NOW()
            )
        );
    END IF;
    RETURN OLD;
END
$$;

CREATE TRIGGER mailbox_message_retention_audit
    AFTER DELETE ON mailbox_messages
    FOR EACH ROW EXECUTE FUNCTION audit_mailbox_message_deletion();
