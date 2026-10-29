ALTER TABLE domains
    ADD CONSTRAINT domains_id_tenant_id_key UNIQUE (id, tenant_id);

ALTER TABLE mailboxes
    ADD CONSTRAINT mailboxes_id_tenant_id_key UNIQUE (id, tenant_id),
    ADD COLUMN mailbox_kind TEXT NOT NULL DEFAULT 'user'
        CHECK (mailbox_kind IN ('user', 'shared')),
    ADD COLUMN display_name TEXT NOT NULL DEFAULT ''
        CHECK (length(display_name) <= 160),
    ADD COLUMN owner_user_id UUID,
    ADD COLUMN quota_bytes BIGINT NOT NULL DEFAULT 1073741824
        CHECK (quota_bytes > 0),
    ADD COLUMN used_bytes BIGINT NOT NULL DEFAULT 0
        CHECK (used_bytes >= 0 AND used_bytes <= quota_bytes),
    ADD COLUMN provisioned_at TIMESTAMPTZ,
    ADD COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE mailboxes
    DROP CONSTRAINT mailboxes_status_check,
    ADD CONSTRAINT mailboxes_status_check
        CHECK (status IN ('provisioning', 'active', 'suspended', 'deleting', 'disabled'));

ALTER TABLE mailboxes
    ADD CONSTRAINT mailboxes_domain_tenant_fk
        FOREIGN KEY (domain_id, tenant_id) REFERENCES domains (id, tenant_id),
    ADD CONSTRAINT mailboxes_owner_membership_fk
        FOREIGN KEY (tenant_id, owner_user_id)
        REFERENCES tenant_memberships (tenant_id, user_id);

UPDATE mailboxes AS mailbox
SET used_bytes = usage.bytes,
    quota_bytes = GREATEST(mailbox.quota_bytes, usage.bytes, 1)
FROM (
    SELECT folder.mailbox_id, COALESCE(SUM(message.raw_size), 0)::BIGINT AS bytes
    FROM mailbox_folders AS folder
    LEFT JOIN mailbox_messages AS message ON message.folder_id = folder.id
    GROUP BY folder.mailbox_id
) AS usage
WHERE mailbox.id = usage.mailbox_id;

CREATE INDEX mailboxes_tenant_status_idx ON mailboxes (tenant_id, status, created_at DESC);

CREATE TABLE mailbox_aliases (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    mailbox_id UUID NOT NULL,
    domain_id UUID NOT NULL,
    local_part TEXT NOT NULL CHECK (length(local_part) BETWEEN 1 AND 64),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mailbox_alias_mailbox_tenant_fk
        FOREIGN KEY (mailbox_id, tenant_id) REFERENCES mailboxes (id, tenant_id)
        ON DELETE CASCADE,
    CONSTRAINT mailbox_alias_domain_tenant_fk
        FOREIGN KEY (domain_id, tenant_id) REFERENCES domains (id, tenant_id)
        ON DELETE RESTRICT
);

CREATE UNIQUE INDEX mailbox_aliases_address_idx
    ON mailbox_aliases (domain_id, lower(local_part));
CREATE INDEX mailbox_aliases_mailbox_idx
    ON mailbox_aliases (mailbox_id, status);

CREATE TABLE mailbox_addresses (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    mailbox_id UUID NOT NULL,
    domain_id UUID NOT NULL,
    local_part TEXT NOT NULL CHECK (length(local_part) BETWEEN 1 AND 64),
    address_kind TEXT NOT NULL CHECK (address_kind IN ('primary', 'alias')),
    alias_id UUID REFERENCES mailbox_aliases (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mailbox_address_mailbox_tenant_fk
        FOREIGN KEY (mailbox_id, tenant_id) REFERENCES mailboxes (id, tenant_id)
        ON DELETE CASCADE,
    CONSTRAINT mailbox_address_domain_tenant_fk
        FOREIGN KEY (domain_id, tenant_id) REFERENCES domains (id, tenant_id)
        ON DELETE RESTRICT,
    CONSTRAINT mailbox_address_alias_kind_check
        CHECK ((address_kind = 'primary' AND alias_id IS NULL) OR
               (address_kind = 'alias' AND alias_id IS NOT NULL))
);

CREATE UNIQUE INDEX mailbox_addresses_canonical_idx
    ON mailbox_addresses (domain_id, lower(local_part));
CREATE INDEX mailbox_addresses_mailbox_idx
    ON mailbox_addresses (tenant_id, mailbox_id);

INSERT INTO mailbox_addresses
    (tenant_id, mailbox_id, domain_id, local_part, address_kind)
SELECT tenant_id, id, domain_id, local_part, 'primary'
FROM mailboxes;

CREATE FUNCTION sync_primary_mailbox_address() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        DELETE FROM mailbox_addresses
        WHERE mailbox_id = OLD.id AND address_kind = 'primary';
        RETURN OLD;
    END IF;

    DELETE FROM mailbox_addresses
    WHERE mailbox_id = NEW.id AND address_kind = 'primary';
    INSERT INTO mailbox_addresses
        (tenant_id, mailbox_id, domain_id, local_part, address_kind)
    VALUES (NEW.tenant_id, NEW.id, NEW.domain_id, NEW.local_part, 'primary');
    RETURN NEW;
END
$$;

CREATE TRIGGER mailboxes_sync_primary_address
    AFTER INSERT OR UPDATE OF tenant_id, domain_id, local_part OR DELETE ON mailboxes
    FOR EACH ROW EXECUTE FUNCTION sync_primary_mailbox_address();

CREATE FUNCTION sync_mailbox_alias_address() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM mailbox_addresses WHERE alias_id = NEW.id;
    INSERT INTO mailbox_addresses
        (tenant_id, mailbox_id, domain_id, local_part, address_kind, alias_id)
    VALUES (NEW.tenant_id, NEW.mailbox_id, NEW.domain_id, NEW.local_part, 'alias', NEW.id);
    RETURN NEW;
END
$$;

CREATE TRIGGER mailbox_aliases_sync_address
    AFTER INSERT OR UPDATE OF tenant_id, mailbox_id, domain_id, local_part ON mailbox_aliases
    FOR EACH ROW EXECUTE FUNCTION sync_mailbox_alias_address();

CREATE TABLE mailbox_permissions (
    tenant_id UUID NOT NULL,
    mailbox_id UUID NOT NULL,
    user_id UUID NOT NULL,
    permission TEXT NOT NULL CHECK (permission IN ('read', 'read_write', 'manage')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, user_id),
    CONSTRAINT mailbox_permission_mailbox_tenant_fk
        FOREIGN KEY (mailbox_id, tenant_id) REFERENCES mailboxes (id, tenant_id)
        ON DELETE CASCADE,
    CONSTRAINT mailbox_permission_membership_fk
        FOREIGN KEY (tenant_id, user_id) REFERENCES tenant_memberships (tenant_id, user_id)
        ON DELETE CASCADE
);

CREATE INDEX mailbox_permissions_user_idx ON mailbox_permissions (tenant_id, user_id);

ALTER TABLE mailbox_folders
    ADD COLUMN parent_id UUID,
    ADD COLUMN special_use TEXT,
    ADD COLUMN uid_next BIGINT NOT NULL DEFAULT 1 CHECK (uid_next > 0),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD CONSTRAINT mailbox_folders_mailbox_id_id_key UNIQUE (mailbox_id, id),
    ADD CONSTRAINT mailbox_folders_parent_same_mailbox_fk
        FOREIGN KEY (mailbox_id, parent_id)
        REFERENCES mailbox_folders (mailbox_id, id) ON DELETE CASCADE,
    ADD CONSTRAINT mailbox_folders_not_own_parent CHECK (parent_id IS NULL OR parent_id <> id),
    ADD CONSTRAINT mailbox_folders_special_use_check
        CHECK (special_use IS NULL OR special_use IN
            ('\Sent', '\Drafts', '\Trash', '\Archive', '\Junk'));

ALTER TABLE mailbox_folders
    DROP CONSTRAINT mailbox_folders_mailbox_id_name_key;

CREATE UNIQUE INDEX mailbox_folders_root_name_idx
    ON mailbox_folders (mailbox_id, name) WHERE parent_id IS NULL;
CREATE UNIQUE INDEX mailbox_folders_child_name_idx
    ON mailbox_folders (mailbox_id, parent_id, name) WHERE parent_id IS NOT NULL;
CREATE UNIQUE INDEX mailbox_folders_special_use_idx
    ON mailbox_folders (mailbox_id, special_use) WHERE special_use IS NOT NULL;
CREATE INDEX mailbox_folders_parent_idx ON mailbox_folders (mailbox_id, parent_id, name);

UPDATE mailbox_folders AS folder
SET uid_next = COALESCE(
    (SELECT MAX(message.uid) + 1
     FROM mailbox_messages AS message
     WHERE message.folder_id = folder.id),
    1
);

CREATE SEQUENCE mailbox_uid_validity_seq AS BIGINT MINVALUE 1;
SELECT setval(
    'mailbox_uid_validity_seq',
    GREATEST(COALESCE(MAX(uid_validity), 1), 1)
)
FROM mailbox_folders;

ALTER TABLE mailbox_messages
    ADD COLUMN header_message_id TEXT,
    ADD COLUMN in_reply_to TEXT,
    ADD COLUMN references_ids TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN subject TEXT,
    ADD COLUMN sent_at TIMESTAMPTZ,
    ADD COLUMN conversation_id UUID,
    ADD COLUMN content_sha256 BYTEA
        CHECK (content_sha256 IS NULL OR octet_length(content_sha256) = 32);

CREATE INDEX mailbox_messages_thread_idx
    ON mailbox_messages (conversation_id, internal_date, id)
    WHERE conversation_id IS NOT NULL;
CREATE INDEX mailbox_messages_message_id_idx
    ON mailbox_messages (header_message_id)
    WHERE header_message_id IS NOT NULL;

CREATE TABLE mailbox_message_labels (
    message_id UUID NOT NULL REFERENCES mailbox_messages (id) ON DELETE CASCADE,
    label TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (message_id, label)
);
