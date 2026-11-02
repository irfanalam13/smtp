-- Tombstone table for async hard deletion of messages
CREATE TABLE mailbox_message_tombstones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    mailbox_id UUID NOT NULL REFERENCES mailbox_folders(mailbox_id) ON DELETE CASCADE,
    message_id UUID NOT NULL,
    recipient TEXT NOT NULL,
    tombstoned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    legal_hold BOOLEAN NOT NULL DEFAULT FALSE,
    deleter_user_id UUID,
    reason TEXT,
    INDEX tenant_id (tenant_id),
    INDEX mailbox_id (mailbox_id),
    INDEX tombstoned_at (tombstoned_at)
) ;
