ALTER TABLE mail_messages
    ADD COLUMN message_schema_version SMALLINT NOT NULL DEFAULT 1
        CHECK (message_schema_version = 1),
    ADD COLUMN parsed_metadata JSONB NOT NULL DEFAULT '{}'::JSONB
        CHECK (jsonb_typeof(parsed_metadata) = 'object'
               AND pg_column_size(parsed_metadata) <= 1048576);

ALTER TABLE inbound_messages
    ADD COLUMN message_schema_version SMALLINT NOT NULL DEFAULT 1
        CHECK (message_schema_version = 1),
    ADD COLUMN parsed_metadata JSONB NOT NULL DEFAULT '{}'::JSONB
        CHECK (jsonb_typeof(parsed_metadata) = 'object'
               AND pg_column_size(parsed_metadata) <= 1048576);

ALTER TABLE mailbox_messages
    ADD COLUMN message_schema_version SMALLINT NOT NULL DEFAULT 1
        CHECK (message_schema_version = 1),
    ADD COLUMN parsed_metadata JSONB NOT NULL DEFAULT '{}'::JSONB
        CHECK (jsonb_typeof(parsed_metadata) = 'object'
               AND pg_column_size(parsed_metadata) <= 1048576);
