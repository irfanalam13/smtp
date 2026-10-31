ALTER TABLE mailbox_messages
    DROP COLUMN parsed_metadata,
    DROP COLUMN message_schema_version;

ALTER TABLE inbound_messages
    DROP COLUMN parsed_metadata,
    DROP COLUMN message_schema_version;

ALTER TABLE mail_messages
    DROP COLUMN parsed_metadata,
    DROP COLUMN message_schema_version;
