CREATE TABLE IF NOT EXISTS mailbox_folders (
    id UUID PRIMARY KEY,
    mailbox_id UUID NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 255),
    uid_validity BIGINT NOT NULL CHECK (uid_validity > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (mailbox_id, name)
);

CREATE TABLE IF NOT EXISTS mailbox_messages (
    id UUID PRIMARY KEY,
    folder_id UUID NOT NULL REFERENCES mailbox_folders(id) ON DELETE CASCADE,
    uid BIGINT NOT NULL CHECK (uid > 0),
    raw_message BYTEA NOT NULL,
    raw_size BIGINT NOT NULL CHECK (raw_size >= 0),
    flags TEXT[] NOT NULL DEFAULT '{}',
    internal_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (folder_id, uid)
);

CREATE INDEX IF NOT EXISTS mailbox_messages_folder_uid_idx
    ON mailbox_messages (folder_id, uid);
CREATE INDEX IF NOT EXISTS mailbox_messages_folder_date_idx
    ON mailbox_messages (folder_id, internal_date DESC, uid);

CREATE TABLE IF NOT EXISTS mailbox_message_keywords (
    message_id UUID NOT NULL REFERENCES mailbox_messages(id) ON DELETE CASCADE,
    keyword TEXT NOT NULL CHECK (length(keyword) BETWEEN 1 AND 64),
    PRIMARY KEY (message_id, keyword)
);
