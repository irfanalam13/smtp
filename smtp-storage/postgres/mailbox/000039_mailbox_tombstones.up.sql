CREATE TABLE mailbox_message_tombstones (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    mailbox_id UUID NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
    message_id UUID NOT NULL,
    folder_id UUID,
    uid BIGINT,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deletion_reason TEXT NOT NULL DEFAULT 'message_deleted'
        CHECK (length(deletion_reason) BETWEEN 1 AND 64),
    PRIMARY KEY (tenant_id, mailbox_id, message_id)
);

CREATE INDEX mailbox_message_tombstones_tenant_idx
    ON mailbox_message_tombstones (tenant_id, deleted_at DESC);

CREATE FUNCTION record_mailbox_message_tombstone() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    target_mailbox_id UUID;
    target_tenant_id UUID;
BEGIN
    SELECT folder.mailbox_id, mailbox.tenant_id
    INTO target_mailbox_id, target_tenant_id
    FROM mailbox_folders AS folder
    JOIN mailboxes AS mailbox ON mailbox.id = folder.mailbox_id
    WHERE folder.id = OLD.folder_id;

    IF target_mailbox_id IS NOT NULL AND target_tenant_id IS NOT NULL THEN
        INSERT INTO mailbox_message_tombstones
            (tenant_id, mailbox_id, message_id, folder_id, uid)
        VALUES
            (target_tenant_id, target_mailbox_id, OLD.id, OLD.folder_id, OLD.uid)
        ON CONFLICT (tenant_id, mailbox_id, message_id) DO UPDATE
        SET deleted_at = EXCLUDED.deleted_at,
            folder_id = EXCLUDED.folder_id,
            uid = EXCLUDED.uid;
    END IF;
    RETURN OLD;
END
$$;

CREATE TRIGGER mailbox_message_tombstone
    AFTER DELETE ON mailbox_messages
    FOR EACH ROW EXECUTE FUNCTION record_mailbox_message_tombstone();
