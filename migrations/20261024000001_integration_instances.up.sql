CREATE TABLE integration_instances (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    client_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('odoo_json2', 'odoo_jsonrpc', 'n8n')),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    environment TEXT NOT NULL CHECK (environment IN ('production', 'staging', 'development')),
    base_url TEXT NOT NULL CHECK (char_length(base_url) <= 2048),
    database TEXT,
    login TEXT,
    secret_nonce BYTEA NOT NULL,
    secret_ciphertext BYTEA NOT NULL,
    secret_preview TEXT NOT NULL,
    server_version TEXT,
    checked_at TIMESTAMPTZ,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ,
    FOREIGN KEY (organization_id, client_id) REFERENCES clients (organization_id, id),
    UNIQUE (client_id, id),
    CONSTRAINT integration_instances_jsonrpc_needs_database_and_login
        CHECK (kind <> 'odoo_jsonrpc' OR (database IS NOT NULL AND login IS NOT NULL)),
    CONSTRAINT integration_instances_n8n_has_no_database_or_login
        CHECK (kind <> 'n8n' OR (database IS NULL AND login IS NULL))
);

CREATE INDEX integration_instances_client ON integration_instances (organization_id, client_id);

CREATE UNIQUE INDEX integration_instances_active_name
    ON integration_instances (client_id, lower(name))
    WHERE archived_at IS NULL;
