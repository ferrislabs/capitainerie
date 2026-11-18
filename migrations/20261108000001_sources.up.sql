CREATE TABLE sources (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    slug TEXT NOT NULL CHECK (char_length(slug) BETWEEN 1 AND 60),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    kind TEXT NOT NULL CHECK (kind IN ('webhook')),
    description TEXT NOT NULL DEFAULT '' CHECK (char_length(description) <= 5000),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    config JSONB NOT NULL DEFAULT '{}'
        CHECK (jsonb_typeof(config) = 'object' AND octet_length(config::text) <= 65536),
    webhook_token_hash TEXT NOT NULL CHECK (char_length(webhook_token_hash) = 64),
    default_agent_id UUID REFERENCES agents (id) ON DELETE SET NULL,
    project_id UUID,
    auto_run BOOLEAN NOT NULL DEFAULT FALSE,
    allow_writes BOOLEAN NOT NULL DEFAULT FALSE,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT sources_slug_unique UNIQUE (organization_id, slug),
    CONSTRAINT sources_webhook_token_unique UNIQUE (webhook_token_hash),
    CONSTRAINT sources_project_same_organization
        FOREIGN KEY (organization_id, project_id) REFERENCES projects (organization_id, id)
);

CREATE TABLE source_events (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    source_id UUID NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    received_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'received'
        CHECK (status IN ('received', 'card_created', 'ignored', 'failed')),
    -- The API refuses a body over 256 KiB; jsonb's text form adds spaces, hence the margin.
    payload JSONB NOT NULL CHECK (octet_length(payload::text) <= 524288),
    summary TEXT NOT NULL DEFAULT '' CHECK (char_length(summary) <= 200),
    card_id UUID REFERENCES cards (id) ON DELETE SET NULL,
    run_id UUID REFERENCES runs (id) ON DELETE SET NULL,
    error TEXT CHECK (char_length(error) <= 2000)
);

CREATE INDEX source_events_source_received ON source_events (source_id, received_at DESC, id DESC);
