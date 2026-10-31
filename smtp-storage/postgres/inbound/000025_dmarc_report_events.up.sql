CREATE TABLE dmarc_report_events (
    id UUID PRIMARY KEY,
    event_fingerprint BYTEA NOT NULL UNIQUE
        CHECK (octet_length(event_fingerprint) = 32),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source_ip INET NOT NULL,
    policy_domain TEXT NOT NULL CHECK (length(policy_domain) BETWEEN 1 AND 253),
    header_from TEXT NOT NULL CHECK (length(header_from) BETWEEN 1 AND 253),
    envelope_from_domain TEXT NOT NULL CHECK (length(envelope_from_domain) <= 253),
    spf_domain TEXT NOT NULL CHECK (length(spf_domain) <= 253),
    spf_scope TEXT NOT NULL CHECK (spf_scope IN ('mfrom', 'helo')),
    spf_result TEXT NOT NULL CHECK (spf_result IN
        ('pass', 'fail', 'softfail', 'neutral', 'temperror', 'permerror', 'none')),
    dmarc_result TEXT NOT NULL CHECK (dmarc_result IN
        ('pass', 'fail', 'temperror', 'permerror', 'none')),
    dmarc_spf_result TEXT NOT NULL CHECK (dmarc_spf_result IN
        ('pass', 'fail', 'temperror', 'permerror', 'none')),
    dmarc_dkim_result TEXT NOT NULL CHECK (dmarc_dkim_result IN
        ('pass', 'fail', 'temperror', 'permerror', 'none')),
    disposition TEXT NOT NULL CHECK (disposition IN ('none', 'quarantine', 'reject')),
    published_policy TEXT NOT NULL CHECK (published_policy IN ('none', 'quarantine', 'reject')),
    subdomain_policy TEXT NOT NULL CHECK (subdomain_policy IN ('none', 'quarantine', 'reject')),
    spf_alignment CHAR(1) NOT NULL CHECK (spf_alignment IN ('r', 's')),
    dkim_alignment CHAR(1) NOT NULL CHECK (dkim_alignment IN ('r', 's')),
    dkim_results JSONB NOT NULL CHECK (jsonb_typeof(dkim_results) = 'array'),
    report_destinations TEXT[] NOT NULL CHECK (cardinality(report_destinations) > 0),
    report_destination_max_bytes BIGINT[] NOT NULL
        CHECK (cardinality(report_destination_max_bytes) = cardinality(report_destinations)
               AND 0 <= ALL(report_destination_max_bytes)),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '90 days'
);

CREATE INDEX dmarc_report_events_period_idx
    ON dmarc_report_events (received_at, policy_domain, source_ip);

CREATE TABLE dmarc_report_batches (
    id UUID PRIMARY KEY,
    period_begin BIGINT NOT NULL,
    period_end BIGINT NOT NULL CHECK (period_end > period_begin),
    policy_domain TEXT NOT NULL CHECK (length(policy_domain) BETWEEN 1 AND 253),
    report_destination TEXT NOT NULL CHECK (length(report_destination) BETWEEN 3 AND 320),
    published_policy TEXT NOT NULL CHECK (published_policy IN ('none', 'quarantine', 'reject')),
    subdomain_policy TEXT NOT NULL CHECK (subdomain_policy IN ('none', 'quarantine', 'reject')),
    spf_alignment CHAR(1) NOT NULL CHECK (spf_alignment IN ('r', 's')),
    dkim_alignment CHAR(1) NOT NULL CHECK (dkim_alignment IN ('r', 's')),
    attachment_name TEXT NOT NULL,
    attachment_gzip BYTEA NOT NULL CHECK (octet_length(attachment_gzip) <= 10485760),
    part_number INTEGER NOT NULL CHECK (part_number >= 0),
    queued_at TIMESTAMPTZ,
    discarded_at TIMESTAMPTZ,
    discard_reason TEXT CHECK (discard_reason IS NULL OR length(discard_reason) <= 128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '90 days',
    UNIQUE (period_begin, period_end, policy_domain, report_destination,
            published_policy, subdomain_policy, spf_alignment, dkim_alignment,
            part_number)
);

CREATE INDEX dmarc_report_batches_ready_idx
    ON dmarc_report_batches (created_at, id)
    WHERE queued_at IS NULL AND discarded_at IS NULL;

CREATE INDEX dmarc_report_events_retention_idx
    ON dmarc_report_events (expires_at);

CREATE TABLE dmarc_report_periods (
    period_begin BIGINT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE dmarc_received_reports (
    id UUID PRIMARY KEY,
    report_fingerprint BYTEA NOT NULL UNIQUE
        CHECK (octet_length(report_fingerprint) = 32),
    report_id TEXT NOT NULL CHECK (length(report_id) BETWEEN 1 AND 255),
    reporter_domain TEXT NOT NULL CHECK (length(reporter_domain) BETWEEN 1 AND 253),
    policy_domain TEXT NOT NULL CHECK (length(policy_domain) BETWEEN 1 AND 253),
    period_begin BIGINT NOT NULL,
    period_end BIGINT NOT NULL CHECK (period_end > period_begin),
    record_count INTEGER NOT NULL CHECK (record_count BETWEEN 1 AND 50000),
    parsed_report JSONB NOT NULL CHECK (jsonb_typeof(parsed_report) = 'object'),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '90 days'
);

CREATE INDEX dmarc_received_reports_retention_idx
    ON dmarc_received_reports (expires_at);
