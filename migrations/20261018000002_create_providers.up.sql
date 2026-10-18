CREATE TABLE provider_credentials (
    organization_id UUID NOT NULL REFERENCES organizations (id),
    kind TEXT NOT NULL CHECK (kind IN ('anthropic', 'mistral', 'openai_compatible')),
    nonce BYTEA NOT NULL,
    ciphertext BYTEA NOT NULL,
    base_url TEXT,
    preview TEXT NOT NULL,
    created_by TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, kind)
);

CREATE TABLE model_assignments (
    organization_id UUID PRIMARY KEY REFERENCES organizations (id),
    assignments JSONB NOT NULL,
    updated_by TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
