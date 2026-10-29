DROP TABLE IF EXISTS mailbox_search_index_state;

DROP INDEX IF EXISTS mailbox_messages_message_id_live_idx;
DROP INDEX IF EXISTS mailbox_messages_recipients_live_idx;
DROP INDEX IF EXISTS mailbox_messages_subject_trgm_live_idx;
DROP INDEX IF EXISTS mailbox_messages_sender_trgm_live_idx;
DROP INDEX IF EXISTS mailbox_messages_sender_live_idx;
DROP INDEX IF EXISTS mailbox_messages_folder_internal_date_live_idx;
DROP INDEX IF EXISTS mailbox_messages_search_vector_idx;

ALTER TABLE mailbox_messages
    DROP COLUMN IF EXISTS search_vector,
    DROP COLUMN IF EXISTS attachment_names_search_text,
    DROP COLUMN IF EXISTS attachment_names,
    DROP COLUMN IF EXISTS body_text,
    DROP COLUMN IF EXISTS recipients_search_text,
    DROP COLUMN IF EXISTS recipients,
    DROP COLUMN IF EXISTS sender_address;
