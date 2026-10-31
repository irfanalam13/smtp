CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE mailbox_messages
    ADD COLUMN sender_address TEXT,
    ADD COLUMN recipients TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN recipients_search_text TEXT NOT NULL DEFAULT '',
    ADD COLUMN body_text TEXT,
    ADD COLUMN attachment_names TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN attachment_names_search_text TEXT NOT NULL DEFAULT '',
    ADD COLUMN search_vector TSVECTOR GENERATED ALWAYS AS (
        setweight(to_tsvector('simple'::regconfig, COALESCE(subject, '')), 'A') ||
        setweight(to_tsvector('simple'::regconfig, COALESCE(sender_address, '')), 'B') ||
        setweight(to_tsvector('simple'::regconfig, recipients_search_text), 'B') ||
        setweight(to_tsvector('simple'::regconfig, COALESCE(body_text, '')), 'C') ||
        setweight(to_tsvector('simple'::regconfig, attachment_names_search_text), 'D')
    ) STORED;

CREATE INDEX mailbox_messages_search_vector_idx
    ON mailbox_messages USING GIN (search_vector)
    WHERE deleted_at IS NULL;
CREATE INDEX mailbox_messages_folder_internal_date_live_idx
    ON mailbox_messages (folder_id, internal_date DESC, uid DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX mailbox_messages_sender_live_idx
    ON mailbox_messages (folder_id, lower(sender_address), internal_date DESC)
    WHERE deleted_at IS NULL AND sender_address IS NOT NULL;
CREATE INDEX mailbox_messages_sender_trgm_live_idx
    ON mailbox_messages USING GIN (sender_address gin_trgm_ops)
    WHERE deleted_at IS NULL AND sender_address IS NOT NULL;
CREATE INDEX mailbox_messages_subject_trgm_live_idx
    ON mailbox_messages USING GIN (subject gin_trgm_ops)
    WHERE deleted_at IS NULL AND subject IS NOT NULL;
CREATE INDEX mailbox_messages_recipients_live_idx
    ON mailbox_messages USING GIN (recipients)
    WHERE deleted_at IS NULL;
CREATE INDEX mailbox_messages_message_id_live_idx
    ON mailbox_messages (header_message_id)
    WHERE deleted_at IS NULL AND header_message_id IS NOT NULL;

CREATE TABLE mailbox_search_index_state (
    mailbox_id UUID PRIMARY KEY REFERENCES mailboxes (id) ON DELETE CASCADE,
    indexed_through_uid BIGINT NOT NULL DEFAULT 0 CHECK (indexed_through_uid >= 0),
    last_indexed_at TIMESTAMPTZ,
    last_error TEXT CHECK (last_error IS NULL OR length(last_error) <= 500),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
