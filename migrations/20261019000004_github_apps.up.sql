-- The GitHub App each organization created from the console (manifest flow).
-- Every secret is sealed with SECRETS_KEY; the associated data is
-- `<organization id>/github_app/<field>`.
CREATE TABLE github_apps (
    organization_id UUID PRIMARY KEY REFERENCES organizations (id),
    app_id BIGINT NOT NULL UNIQUE,
    slug TEXT NOT NULL,
    client_id TEXT NOT NULL,
    html_url TEXT NOT NULL,
    owner_login TEXT NOT NULL,
    owner_type TEXT NOT NULL,
    client_secret_nonce BYTEA NOT NULL,
    client_secret_ciphertext BYTEA NOT NULL,
    private_key_nonce BYTEA NOT NULL,
    private_key_ciphertext BYTEA NOT NULL,
    webhook_secret_nonce BYTEA NOT NULL,
    webhook_secret_ciphertext BYTEA NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
