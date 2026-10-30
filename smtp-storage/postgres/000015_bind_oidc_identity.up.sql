ALTER TABLE users
    ADD COLUMN identity_issuer TEXT NOT NULL DEFAULT ''
        CHECK (length(identity_issuer) <= 2048);
