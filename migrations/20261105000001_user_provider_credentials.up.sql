CREATE TABLE user_provider_credentials (
    organization_id UUID NOT NULL REFERENCES organizations (id),
    subject TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('anthropic', 'mistral', 'openai_compatible')),
    nonce BYTEA NOT NULL,
    ciphertext BYTEA NOT NULL,
    key_preview TEXT NOT NULL,
    base_url TEXT,
    default_model TEXT NOT NULL CHECK (default_model <> ''),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, subject, kind)
);

CREATE UNIQUE INDEX user_provider_credentials_one_default
    ON user_provider_credentials (organization_id, subject)
    WHERE is_default;

ALTER TABLE agent_tasks
    ADD COLUMN provider TEXT CHECK (provider IN ('anthropic', 'mistral', 'openai_compatible')),
    ADD COLUMN model TEXT;
