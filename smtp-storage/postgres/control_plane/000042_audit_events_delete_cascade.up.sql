-- Fix audit_events foreign key for proper cascade deletion of tenants
ALTER TABLE audit_events
    DROP CONSTRAINT audit_events_tenant_id_fkey,
    ADD CONSTRAINT audit_events_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;
