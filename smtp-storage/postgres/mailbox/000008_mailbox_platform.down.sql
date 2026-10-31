DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM mailbox_folders WHERE parent_id IS NOT NULL) THEN
        RAISE EXCEPTION 'cannot downgrade mailbox hierarchy while nested folders exist';
    END IF;
END
$$;

DROP TABLE IF EXISTS mailbox_message_labels;

DROP INDEX IF EXISTS mailbox_messages_message_id_idx;
DROP INDEX IF EXISTS mailbox_messages_thread_idx;

ALTER TABLE mailbox_messages
    DROP COLUMN IF EXISTS content_sha256,
    DROP COLUMN IF EXISTS conversation_id,
    DROP COLUMN IF EXISTS sent_at,
    DROP COLUMN IF EXISTS subject,
    DROP COLUMN IF EXISTS references_ids,
    DROP COLUMN IF EXISTS in_reply_to,
    DROP COLUMN IF EXISTS header_message_id;

DROP INDEX IF EXISTS mailbox_folders_parent_idx;
DROP INDEX IF EXISTS mailbox_folders_special_use_idx;
DROP INDEX IF EXISTS mailbox_folders_child_name_idx;
DROP INDEX IF EXISTS mailbox_folders_root_name_idx;

ALTER TABLE mailbox_folders
    DROP CONSTRAINT IF EXISTS mailbox_folders_special_use_check,
    DROP CONSTRAINT IF EXISTS mailbox_folders_not_own_parent,
    DROP CONSTRAINT IF EXISTS mailbox_folders_parent_same_mailbox_fk,
    DROP CONSTRAINT IF EXISTS mailbox_folders_mailbox_id_id_key,
    DROP COLUMN IF EXISTS updated_at,
    DROP COLUMN IF EXISTS uid_next,
    DROP COLUMN IF EXISTS special_use,
    DROP COLUMN IF EXISTS parent_id;

ALTER TABLE mailbox_folders
    ADD CONSTRAINT mailbox_folders_mailbox_id_name_key UNIQUE (mailbox_id, name);

DROP SEQUENCE IF EXISTS mailbox_uid_validity_seq;

DROP INDEX IF EXISTS mailbox_permissions_user_idx;
DROP TABLE IF EXISTS mailbox_permissions;
DROP TRIGGER IF EXISTS mailbox_aliases_sync_address ON mailbox_aliases;
DROP FUNCTION IF EXISTS sync_mailbox_alias_address();
DROP TRIGGER IF EXISTS mailboxes_sync_primary_address ON mailboxes;
DROP FUNCTION IF EXISTS sync_primary_mailbox_address();
DROP INDEX IF EXISTS mailbox_addresses_mailbox_idx;
DROP INDEX IF EXISTS mailbox_addresses_canonical_idx;
DROP TABLE IF EXISTS mailbox_addresses;
DROP INDEX IF EXISTS mailbox_aliases_mailbox_idx;
DROP INDEX IF EXISTS mailbox_aliases_address_idx;
DROP TABLE IF EXISTS mailbox_aliases;
DROP INDEX IF EXISTS mailboxes_tenant_status_idx;

ALTER TABLE mailboxes
    DROP CONSTRAINT IF EXISTS mailboxes_owner_membership_fk,
    DROP CONSTRAINT IF EXISTS mailboxes_domain_tenant_fk,
    DROP CONSTRAINT IF EXISTS mailboxes_status_check,
    DROP CONSTRAINT IF EXISTS mailboxes_id_tenant_id_key,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS provisioned_at,
    DROP COLUMN IF EXISTS used_bytes,
    DROP COLUMN IF EXISTS quota_bytes,
    DROP COLUMN IF EXISTS owner_user_id,
    DROP COLUMN IF EXISTS display_name,
    DROP COLUMN IF EXISTS mailbox_kind;

ALTER TABLE mailboxes
    ADD CONSTRAINT mailboxes_status_check CHECK (status IN ('active', 'disabled'));

ALTER TABLE domains
    DROP CONSTRAINT IF EXISTS domains_id_tenant_id_key;
