CREATE TABLE mcp_connectors (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    slug TEXT NOT NULL CHECK (char_length(slug) BETWEEN 1 AND 60),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    url TEXT NOT NULL CHECK (char_length(url) BETWEEN 1 AND 2048),
    resource TEXT NOT NULL CHECK (char_length(resource) BETWEEN 1 AND 2048),
    issuer TEXT NOT NULL CHECK (char_length(issuer) BETWEEN 1 AND 2048),
    authorization_endpoint TEXT NOT NULL CHECK (char_length(authorization_endpoint) BETWEEN 1 AND 2048),
    token_endpoint TEXT NOT NULL CHECK (char_length(token_endpoint) BETWEEN 1 AND 2048),
    client_id TEXT NOT NULL CHECK (char_length(client_id) BETWEEN 1 AND 512),
    scopes TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(scopes) <= 100),
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT mcp_connectors_slug_unique UNIQUE (organization_id, slug)
);

CREATE TABLE mcp_connections (
    connector_id UUID NOT NULL REFERENCES mcp_connectors (id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    access_nonce BYTEA NOT NULL,
    access_ciphertext BYTEA NOT NULL,
    refresh_nonce BYTEA,
    refresh_ciphertext BYTEA,
    expires_at TIMESTAMPTZ,
    scope TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (connector_id, subject),
    CONSTRAINT mcp_connections_refresh_sealed
        CHECK ((refresh_nonce IS NULL) = (refresh_ciphertext IS NULL))
);

CREATE INDEX mcp_connections_subject ON mcp_connections (subject);

CREATE TABLE mcp_oauth_states (
    state TEXT PRIMARY KEY,
    connector_id UUID NOT NULL REFERENCES mcp_connectors (id) ON DELETE CASCADE,
    subject TEXT NOT NULL,
    verifier_nonce BYTEA NOT NULL,
    verifier_ciphertext BYTEA NOT NULL,
    redirect_uri TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX mcp_oauth_states_expires_at ON mcp_oauth_states (expires_at);
