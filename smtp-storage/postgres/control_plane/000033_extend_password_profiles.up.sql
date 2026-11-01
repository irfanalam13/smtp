ALTER TABLE users
    ADD COLUMN first_name TEXT NOT NULL DEFAULT 'User'
        CHECK (length(first_name) BETWEEN 2 AND 50),
    ADD COLUMN last_name TEXT NOT NULL DEFAULT 'Account'
        CHECK (length(last_name) BETWEEN 2 AND 50),
    ADD COLUMN phone_number TEXT
        CHECK (phone_number IS NULL OR length(phone_number) BETWEEN 7 AND 32),
    ADD COLUMN country TEXT
        CHECK (country IS NULL OR length(country) BETWEEN 2 AND 100),
    ADD COLUMN organization_name TEXT
        CHECK (organization_name IS NULL OR length(organization_name) BETWEEN 1 AND 160);
