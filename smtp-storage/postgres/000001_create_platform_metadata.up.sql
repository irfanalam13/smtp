CREATE TABLE IF NOT EXISTS schema_metadata (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    platform_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO schema_metadata (id, platform_name)
VALUES (1, 'smtp-platform')
ON CONFLICT (id) DO NOTHING;
