CREATE FUNCTION record_mailbox_metadata_change() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    old_row JSONB;
    new_row JSONB;
    target_mailbox_id UUID;
    target_message_id UUID;
    target_label_id UUID;
    target_tenant_id UUID;
    metadata_kind TEXT;
    outbox_key TEXT;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        old_row := to_jsonb(OLD);
    END IF;
    IF TG_OP <> 'DELETE' THEN
        new_row := to_jsonb(NEW);
    END IF;

    target_mailbox_id := COALESCE(
        (new_row ->> 'mailbox_id')::UUID,
        (old_row ->> 'mailbox_id')::UUID
    );
    target_message_id := COALESCE(
        (new_row ->> 'message_id')::UUID,
        (old_row ->> 'message_id')::UUID,
        (new_row ->> 'id')::UUID,
        (old_row ->> 'id')::UUID
    );

    IF TG_TABLE_NAME = 'mailbox_message_label_assignments' THEN
        metadata_kind := 'label';
        target_label_id := COALESCE(
            (new_row ->> 'label_id')::UUID,
            (old_row ->> 'label_id')::UUID
        );
    ELSIF TG_TABLE_NAME = 'mailbox_draft_revisions' THEN
        metadata_kind := 'draft';
    ELSE
        metadata_kind := 'custom_properties';
    END IF;

    IF target_mailbox_id IS NULL OR target_message_id IS NULL OR NOT EXISTS (
        SELECT 1 FROM mailbox_messages
        WHERE mailbox_id = target_mailbox_id AND id = target_message_id
    ) THEN
        IF TG_OP = 'DELETE' THEN
            RETURN OLD;
        END IF;
        RETURN NEW;
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
    VALUES (
        target_mailbox_id,
        target_message_id,
        'upsert',
        jsonb_build_object(
            'message_id', target_message_id,
            'metadata_kind', metadata_kind,
            'label_id', target_label_id
        )
    );

    outbox_key := target_message_id::TEXT || ':' || metadata_kind || ':' ||
        COALESCE(target_label_id::TEXT, '') || ':' || txid_current()::TEXT;
    INSERT INTO mailbox_event_outbox
        (tenant_id, mailbox_id, event_type, idempotency_key, payload)
    VALUES (
        target_tenant_id,
        target_mailbox_id,
        'mailbox.message.metadata_changed',
        outbox_key,
        jsonb_build_object(
            'mailbox_id', target_mailbox_id,
            'message_id', target_message_id,
            'metadata_kind', metadata_kind,
            'label_id', target_label_id
        )
    )
    ON CONFLICT (tenant_id, event_type, idempotency_key) DO NOTHING;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER mailbox_label_assignment_change_journal
    AFTER INSERT OR DELETE ON mailbox_message_label_assignments
    FOR EACH ROW EXECUTE FUNCTION record_mailbox_metadata_change();

CREATE TRIGGER mailbox_custom_properties_change_journal
    AFTER UPDATE OF custom_properties ON mailbox_messages
    FOR EACH ROW EXECUTE FUNCTION record_mailbox_metadata_change();

CREATE TRIGGER mailbox_draft_revision_change_journal
    AFTER INSERT ON mailbox_draft_revisions
    FOR EACH ROW EXECUTE FUNCTION record_mailbox_metadata_change();
