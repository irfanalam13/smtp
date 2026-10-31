ALTER TABLE mailbox_messages
    ADD COLUMN mailbox_id UUID;

UPDATE mailbox_messages AS message
SET mailbox_id = folder.mailbox_id
FROM mailbox_folders AS folder
WHERE folder.id = message.folder_id;

ALTER TABLE mailbox_messages
    ALTER COLUMN mailbox_id SET NOT NULL,
    ADD CONSTRAINT mailbox_messages_mailbox_id_id_key UNIQUE (mailbox_id, id),
    ADD CONSTRAINT mailbox_messages_folder_mailbox_fk
        FOREIGN KEY (mailbox_id, folder_id)
        REFERENCES mailbox_folders (mailbox_id, id) ON DELETE CASCADE,
    ADD COLUMN custom_properties JSONB NOT NULL DEFAULT '{}'::JSONB
        CHECK (jsonb_typeof(custom_properties) = 'object'
               AND pg_column_size(custom_properties) <= 16384);

CREATE TABLE mailbox_conversations (
    mailbox_id UUID NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    id UUID NOT NULL,
    normalized_subject TEXT,
    latest_message_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, id)
);

INSERT INTO mailbox_conversations (mailbox_id, id, normalized_subject, latest_message_at)
SELECT message.mailbox_id, message.conversation_id,
       MIN(message.subject), MAX(message.internal_date)
FROM mailbox_messages AS message
WHERE message.conversation_id IS NOT NULL
GROUP BY message.mailbox_id, message.conversation_id;

ALTER TABLE mailbox_messages
    ADD CONSTRAINT mailbox_messages_conversation_mailbox_fk
        FOREIGN KEY (mailbox_id, conversation_id)
        REFERENCES mailbox_conversations (mailbox_id, id) ON DELETE SET NULL (conversation_id);

CREATE INDEX mailbox_conversations_latest_idx
    ON mailbox_conversations (mailbox_id, latest_message_at DESC, id);

CREATE TABLE mailbox_labels (
    id UUID PRIMARY KEY,
    mailbox_id UUID NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 64),
    label_kind TEXT NOT NULL DEFAULT 'label'
        CHECK (label_kind IN ('label', 'category', 'tag', 'smart')),
    color TEXT CHECK (color IS NULL OR color ~ '^#[0-9A-Fa-f]{6}$'),
    query_definition JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mailbox_labels_smart_query_check
        CHECK ((label_kind = 'smart' AND query_definition IS NOT NULL
                AND pg_column_size(query_definition) <= 8192)
            OR (label_kind <> 'smart' AND query_definition IS NULL)),
    UNIQUE (mailbox_id, id)
);

CREATE UNIQUE INDEX mailbox_labels_name_idx
    ON mailbox_labels (mailbox_id, lower(name));

CREATE TABLE mailbox_message_label_assignments (
    mailbox_id UUID NOT NULL,
    message_id UUID NOT NULL,
    label_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, message_id, label_id),
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    FOREIGN KEY (mailbox_id, label_id)
        REFERENCES mailbox_labels (mailbox_id, id) ON DELETE CASCADE
);

CREATE INDEX mailbox_message_labels_by_label_idx
    ON mailbox_message_label_assignments (mailbox_id, label_id, message_id);

CREATE FUNCTION prevent_smart_label_assignment() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    assigned_label_kind TEXT;
BEGIN
    SELECT label_kind INTO assigned_label_kind
    FROM mailbox_labels
    WHERE mailbox_id = NEW.mailbox_id AND id = NEW.label_id
    FOR UPDATE;
    IF assigned_label_kind = 'smart' THEN
        RAISE EXCEPTION 'smart labels are derived and cannot be assigned'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER mailbox_message_label_assignment_static_only
    BEFORE INSERT OR UPDATE OF mailbox_id, label_id
    ON mailbox_message_label_assignments
    FOR EACH ROW EXECUTE FUNCTION prevent_smart_label_assignment();

CREATE FUNCTION prevent_assigned_label_becoming_smart() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.label_kind = 'smart' AND EXISTS (
        SELECT 1
        FROM mailbox_message_label_assignments AS assignment
        WHERE assignment.mailbox_id = OLD.mailbox_id
          AND assignment.label_id = OLD.id
    ) THEN
        RAISE EXCEPTION 'a label with assignments cannot become a smart label'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER mailbox_label_smart_reclassification_guard
    BEFORE UPDATE OF label_kind ON mailbox_labels
    FOR EACH ROW EXECUTE FUNCTION prevent_assigned_label_becoming_smart();

INSERT INTO mailbox_labels (id, mailbox_id, name, label_kind)
SELECT gen_random_uuid(), legacy.mailbox_id, legacy.label, 'label'
FROM (
    SELECT DISTINCT message.mailbox_id, old_label.label
    FROM mailbox_message_labels AS old_label
    JOIN mailbox_messages AS message ON message.id = old_label.message_id
) AS legacy
ON CONFLICT (mailbox_id, lower(name)) DO NOTHING;

INSERT INTO mailbox_message_label_assignments (mailbox_id, message_id, label_id)
SELECT message.mailbox_id, old_label.message_id, label.id
FROM mailbox_message_labels AS old_label
JOIN mailbox_messages AS message ON message.id = old_label.message_id
JOIN mailbox_labels AS label
  ON label.mailbox_id = message.mailbox_id
 AND lower(label.name) = lower(old_label.label)
ON CONFLICT DO NOTHING;

CREATE INDEX mailbox_messages_custom_properties_idx
    ON mailbox_messages USING GIN (custom_properties jsonb_path_ops);

CREATE TABLE mailbox_filters (
    id UUID PRIMARY KEY,
    mailbox_id UUID NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 120),
    direction TEXT NOT NULL DEFAULT 'incoming'
        CHECK (direction IN ('incoming', 'outgoing', 'both')),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 1000,
    stop_processing BOOLEAN NOT NULL DEFAULT FALSE,
    conditions JSONB NOT NULL CHECK (
        jsonb_typeof(conditions) = 'object' AND pg_column_size(conditions) <= 32768
    ),
    actions JSONB NOT NULL CHECK (
        CASE WHEN jsonb_typeof(actions) = 'array'
             THEN jsonb_array_length(actions) BETWEEN 1 AND 32
             ELSE FALSE END
        AND pg_column_size(actions) <= 16384
    ),
    version BIGINT NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (mailbox_id, id)
);

CREATE UNIQUE INDEX mailbox_filters_name_idx
    ON mailbox_filters (mailbox_id, lower(name));
CREATE INDEX mailbox_filters_execution_order_idx
    ON mailbox_filters (mailbox_id, direction, enabled, priority, id);

CREATE TABLE mailbox_filter_versions (
    mailbox_id UUID NOT NULL,
    filter_id UUID NOT NULL,
    version BIGINT NOT NULL CHECK (version > 0),
    snapshot JSONB NOT NULL CHECK (
        jsonb_typeof(snapshot) = 'object' AND pg_column_size(snapshot) <= 65536
    ),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, filter_id, version),
    FOREIGN KEY (mailbox_id, filter_id)
        REFERENCES mailbox_filters (mailbox_id, id) ON DELETE CASCADE
);

CREATE TABLE mailbox_rule_executions (
    id UUID PRIMARY KEY,
    mailbox_id UUID NOT NULL,
    filter_id UUID NOT NULL,
    message_id UUID,
    rule_version BIGINT NOT NULL CHECK (rule_version > 0),
    idempotency_key BYTEA NOT NULL CHECK (octet_length(idempotency_key) = 32),
    execution_state TEXT NOT NULL DEFAULT 'planned'
        CHECK (execution_state IN ('planned', 'applied', 'failed', 'skipped')),
    result JSONB NOT NULL DEFAULT '{}'::JSONB
        CHECK (jsonb_typeof(result) = 'object' AND pg_column_size(result) <= 16384),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    FOREIGN KEY (mailbox_id, filter_id)
        REFERENCES mailbox_filters (mailbox_id, id) ON DELETE CASCADE,
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    UNIQUE (mailbox_id, filter_id, idempotency_key)
);

CREATE INDEX mailbox_rule_executions_pending_idx
    ON mailbox_rule_executions (created_at, id)
    WHERE execution_state IN ('planned', 'failed');

CREATE TABLE mailbox_quarantine_entries (
    mailbox_id UUID NOT NULL,
    message_id UUID NOT NULL,
    original_folder_id UUID,
    quarantine_state TEXT NOT NULL DEFAULT 'quarantined'
        CHECK (quarantine_state IN ('quarantined', 'released', 'expired')),
    reason_code TEXT NOT NULL CHECK (length(reason_code) BETWEEN 1 AND 64),
    quarantined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    review_until TIMESTAMPTZ,
    released_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, message_id),
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    FOREIGN KEY (mailbox_id, original_folder_id)
        REFERENCES mailbox_folders (mailbox_id, id) ON DELETE SET NULL (original_folder_id)
);

CREATE INDEX mailbox_quarantine_review_idx
    ON mailbox_quarantine_entries (mailbox_id, review_until, quarantined_at)
    WHERE quarantine_state = 'quarantined';

CREATE TABLE mailbox_trash_entries (
    mailbox_id UUID NOT NULL,
    message_id UUID NOT NULL,
    original_folder_id UUID NOT NULL,
    original_uid BIGINT NOT NULL CHECK (original_uid > 0),
    trashed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    purge_after TIMESTAMPTZ,
    PRIMARY KEY (mailbox_id, message_id),
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    FOREIGN KEY (mailbox_id, original_folder_id)
        REFERENCES mailbox_folders (mailbox_id, id) ON DELETE RESTRICT
);

CREATE INDEX mailbox_trash_expiration_idx
    ON mailbox_trash_entries (purge_after, mailbox_id)
    WHERE purge_after IS NOT NULL;

CREATE TABLE mailbox_archive_entries (
    mailbox_id UUID NOT NULL,
    message_id UUID NOT NULL,
    archive_tier TEXT NOT NULL DEFAULT 'online'
        CHECK (archive_tier IN ('online', 'cold_pending', 'cold', 'restore_pending')),
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retention_until TIMESTAMPTZ,
    legal_hold BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, message_id),
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE
);

CREATE INDEX mailbox_archive_retention_idx
    ON mailbox_archive_entries (retention_until, mailbox_id)
    WHERE legal_hold = FALSE AND retention_until IS NOT NULL;

CREATE TABLE mailbox_drafts (
    mailbox_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    message_id UUID NOT NULL,
    owner_user_id UUID,
    current_version BIGINT NOT NULL DEFAULT 0 CHECK (current_version >= 0),
    updated_by UUID,
    autosaved_at TIMESTAMPTZ,
    PRIMARY KEY (mailbox_id, message_id),
    UNIQUE (mailbox_id, message_id, tenant_id),
    FOREIGN KEY (mailbox_id, tenant_id)
        REFERENCES mailboxes (id, tenant_id) ON DELETE CASCADE,
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, owner_user_id)
        REFERENCES tenant_memberships (tenant_id, user_id) ON DELETE SET NULL (owner_user_id),
    FOREIGN KEY (tenant_id, updated_by)
        REFERENCES tenant_memberships (tenant_id, user_id) ON DELETE SET NULL (updated_by)
);

CREATE TABLE mailbox_draft_revisions (
    mailbox_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    message_id UUID NOT NULL,
    version BIGINT NOT NULL CHECK (version > 0),
    raw_message BYTEA NOT NULL,
    raw_size BIGINT GENERATED ALWAYS AS (octet_length(raw_message)::BIGINT) STORED,
    editor_user_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, message_id, version),
    FOREIGN KEY (mailbox_id, message_id, tenant_id)
        REFERENCES mailbox_drafts (mailbox_id, message_id, tenant_id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, editor_user_id)
        REFERENCES tenant_memberships (tenant_id, user_id) ON DELETE SET NULL (editor_user_id)
);

CREATE FUNCTION charge_mailbox_draft_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    revision_size BIGINT := octet_length(NEW.raw_message)::BIGINT;
BEGIN
    UPDATE mailboxes
    SET used_bytes = used_bytes + revision_size
    WHERE id = NEW.mailbox_id
      AND status = 'active'
      AND used_bytes <= quota_bytes - revision_size;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'mailbox unavailable or draft revision exceeds quota'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END
$$;

CREATE FUNCTION release_mailbox_draft_revision() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE mailboxes
    SET used_bytes = GREATEST(used_bytes - OLD.raw_size, 0)
    WHERE id = OLD.mailbox_id;
    RETURN OLD;
END
$$;

CREATE FUNCTION prevent_mailbox_draft_revision_update() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'draft revisions are immutable; insert a new version'
        USING ERRCODE = 'check_violation';
END
$$;

CREATE TRIGGER mailbox_draft_revision_charge
    BEFORE INSERT ON mailbox_draft_revisions
    FOR EACH ROW EXECUTE FUNCTION charge_mailbox_draft_revision();
CREATE TRIGGER mailbox_draft_revision_release
    BEFORE DELETE ON mailbox_draft_revisions
    FOR EACH ROW EXECUTE FUNCTION release_mailbox_draft_revision();
CREATE TRIGGER mailbox_draft_revision_immutable
    BEFORE UPDATE ON mailbox_draft_revisions
    FOR EACH ROW EXECUTE FUNCTION prevent_mailbox_draft_revision_update();

CREATE TABLE mailbox_sent_submissions (
    id UUID PRIMARY KEY,
    mailbox_id UUID NOT NULL,
    message_id UUID NOT NULL,
    submission_id UUID NOT NULL,
    idempotency_key BYTEA NOT NULL CHECK (octet_length(idempotency_key) = 32),
    queue_id TEXT,
    submission_state TEXT NOT NULL DEFAULT 'queued'
        CHECK (submission_state IN ('queued', 'accepted', 'deferred', 'completed', 'failed')),
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (mailbox_id, message_id)
        REFERENCES mailbox_messages (mailbox_id, id) ON DELETE CASCADE,
    UNIQUE (mailbox_id, submission_id),
    UNIQUE (mailbox_id, idempotency_key)
);

CREATE TABLE mailbox_sent_recipient_events (
    id UUID PRIMARY KEY,
    submission_id UUID NOT NULL REFERENCES mailbox_sent_submissions (id) ON DELETE CASCADE,
    source_event_id UUID NOT NULL UNIQUE,
    recipient_address TEXT NOT NULL CHECK (length(recipient_address) <= 320),
    recipient_state TEXT NOT NULL DEFAULT 'queued'
        CHECK (recipient_state IN ('queued', 'accepted', 'deferred', 'delivered', 'bounced', 'failed')),
    status_code TEXT,
    diagnostic TEXT CHECK (diagnostic IS NULL OR length(diagnostic) <= 1024),
    event_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (submission_id, source_event_id)
);

CREATE TABLE mailbox_sent_recipient_status (
    submission_id UUID NOT NULL REFERENCES mailbox_sent_submissions (id) ON DELETE CASCADE,
    recipient_address TEXT NOT NULL CHECK (length(recipient_address) <= 320),
    recipient_state TEXT NOT NULL
        CHECK (recipient_state IN ('queued', 'accepted', 'deferred', 'delivered', 'bounced', 'failed')),
    latest_event_at TIMESTAMPTZ NOT NULL,
    latest_source_event_id UUID NOT NULL REFERENCES mailbox_sent_recipient_events (source_event_id),
    status_code TEXT,
    diagnostic TEXT CHECK (diagnostic IS NULL OR length(diagnostic) <= 1024),
    PRIMARY KEY (submission_id, recipient_address)
);

CREATE INDEX mailbox_sent_submission_state_idx
    ON mailbox_sent_submissions (mailbox_id, submitted_at DESC, id);
CREATE INDEX mailbox_sent_recipient_state_idx
    ON mailbox_sent_recipient_events (submission_id, event_at, id);
CREATE INDEX mailbox_sent_recipient_status_idx
    ON mailbox_sent_recipient_status (submission_id, recipient_state, recipient_address);

CREATE TABLE mailbox_change_log (
    mailbox_id UUID NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    token BIGINT GENERATED ALWAYS AS IDENTITY,
    message_id UUID NOT NULL,
    change_kind TEXT NOT NULL CHECK (change_kind IN ('upsert', 'delete')),
    payload JSONB NOT NULL CHECK (
        jsonb_typeof(payload) = 'object' AND pg_column_size(payload) <= 8192
    ),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (mailbox_id, token)
);

CREATE INDEX mailbox_change_log_message_idx
    ON mailbox_change_log (mailbox_id, message_id, token DESC);

CREATE TABLE mailbox_event_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
    mailbox_id UUID NOT NULL REFERENCES mailboxes (id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (length(event_type) BETWEEN 1 AND 128),
    idempotency_key TEXT NOT NULL CHECK (length(idempotency_key) BETWEEN 1 AND 160),
    payload JSONB NOT NULL CHECK (
        jsonb_typeof(payload) = 'object' AND pg_column_size(payload) <= 16384
    ),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    locked_by UUID,
    locked_until TIMESTAMPTZ,
    published_at TIMESTAMPTZ,
    last_error TEXT CHECK (last_error IS NULL OR length(last_error) <= 1024),
    UNIQUE (tenant_id, event_type, idempotency_key)
);

CREATE INDEX mailbox_event_outbox_ready_idx
    ON mailbox_event_outbox (available_at, locked_until, created_at, id)
    WHERE published_at IS NULL;

CREATE FUNCTION record_mailbox_message_change() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    target_mailbox_id UUID;
    target_message_id UUID;
    target_folder_id UUID;
    target_uid BIGINT;
    target_flags TEXT[];
    change_kind_value TEXT;
    change_payload JSONB;
    target_tenant_id UUID;
BEGIN
    IF TG_OP = 'DELETE' THEN
        target_mailbox_id := OLD.mailbox_id;
        target_message_id := OLD.id;
        change_kind_value := 'delete';
        change_payload := jsonb_build_object('message_id', OLD.id);
    ELSE
        target_mailbox_id := NEW.mailbox_id;
        target_message_id := NEW.id;
        target_folder_id := NEW.folder_id;
        target_uid := NEW.uid;
        target_flags := NEW.flags;
        change_kind_value := 'upsert';
        change_payload := jsonb_build_object(
            'message_id', NEW.id, 'folder_id', NEW.folder_id,
            'uid', NEW.uid, 'flags', NEW.flags,
            'deleted', NEW.deleted_at IS NOT NULL
        );
    END IF;

    SELECT tenant_id INTO target_tenant_id
    FROM mailboxes WHERE id = target_mailbox_id;

    IF target_tenant_id IS NULL THEN
        IF TG_OP = 'DELETE' THEN
            RETURN OLD;
        END IF;
        RETURN NEW;
    END IF;

    INSERT INTO mailbox_change_log (mailbox_id, message_id, change_kind, payload)
    VALUES (target_mailbox_id, target_message_id, change_kind_value, change_payload);

    INSERT INTO mailbox_event_outbox
        (tenant_id, mailbox_id, event_type, idempotency_key, payload)
    VALUES (
        target_tenant_id,
        target_mailbox_id,
        CASE WHEN TG_OP = 'DELETE' THEN 'mailbox.message.deleted'
             ELSE 'mailbox.message.changed' END,
        target_message_id::TEXT || ':' || COALESCE(target_folder_id::TEXT, '') ||
            ':' || COALESCE(target_uid::TEXT, 'deleted') ||
            ':' || COALESCE(array_to_string(target_flags, ','), '') ||
            ':' || CASE WHEN TG_OP = 'DELETE' THEN 'delete'
                        ELSE COALESCE(NEW.deleted_at::TEXT, 'live') END,
        jsonb_build_object(
            'mailbox_id', target_mailbox_id,
            'message_id', target_message_id,
            'folder_id', target_folder_id,
            'uid', target_uid,
            'change_kind', change_kind_value
        )
    )
    ON CONFLICT (tenant_id, event_type, idempotency_key) DO NOTHING;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER mailbox_message_change_journal
    AFTER INSERT OR UPDATE OF folder_id, uid, flags, deleted_at OR DELETE
    ON mailbox_messages
    FOR EACH ROW EXECUTE FUNCTION record_mailbox_message_change();
