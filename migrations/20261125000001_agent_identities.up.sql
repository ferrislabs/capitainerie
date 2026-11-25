CREATE TABLE agent_identities (
    agent_id UUID PRIMARY KEY REFERENCES agents (id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    client_id TEXT NOT NULL CHECK (char_length(client_id) BETWEEN 1 AND 255),
    client_uuid TEXT CHECK (char_length(client_uuid) BETWEEN 1 AND 255),
    status TEXT NOT NULL CHECK (status IN ('pending', 'active', 'disabled')),
    secret_nonce BYTEA,
    secret_ciphertext BYTEA,
    failure TEXT CHECK (char_length(failure) <= 1000),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT agent_identities_client_unique UNIQUE (organization_id, client_id),
    CONSTRAINT agent_identities_secret_sealed
        CHECK ((secret_nonce IS NULL) = (secret_ciphertext IS NULL)),
    CONSTRAINT agent_identities_active_complete
        CHECK (status <> 'active' OR (client_uuid IS NOT NULL AND secret_nonce IS NOT NULL))
);

CREATE INDEX agent_identities_pending ON agent_identities (updated_at) WHERE status = 'pending';
