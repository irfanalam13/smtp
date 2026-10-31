CREATE UNIQUE INDEX api_keys_tenant_name_unique_idx
    ON api_keys (tenant_id, name);
