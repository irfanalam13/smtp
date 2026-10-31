CREATE TABLE mailbox_append_idempotency (
    tenant_id UUID NOT NULL,
    mailbox_id UUID NOT NULL,
    user_id UUID NOT NULL,
    idempotency_key TEXT NOT NULL
        CHECK (length(idempotency_key) BETWEEN 1 AND 128),
    request_sha256 BYTEA NOT NULL
        CHECK (octet_length(request_sha256) = 32),
    folder_id UUID NOT NULL,
    result_folder_id UUID,
    message_id UUID,
    uid BIGINT CHECK (uid IS NULL OR uid > 0),
    result_flags TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, mailbox_id, user_id, idempotency_key),
    CHECK (
        (message_id IS NULL AND uid IS NULL AND result_folder_id IS NULL AND result_flags IS NULL)
        OR
        (message_id IS NOT NULL AND uid IS NOT NULL AND result_folder_id IS NOT NULL AND result_flags IS NOT NULL)
    ),
    CONSTRAINT mailbox_append_idempotency_mailbox_fk
        FOREIGN KEY (mailbox_id, tenant_id)
        REFERENCES mailboxes (id, tenant_id) ON DELETE CASCADE,
    CONSTRAINT mailbox_append_idempotency_membership_fk
        FOREIGN KEY (tenant_id, user_id)
        REFERENCES tenant_memberships (tenant_id, user_id) ON DELETE CASCADE
);

CREATE INDEX mailbox_append_idempotency_expiry_idx
    ON mailbox_append_idempotency (expires_at);
