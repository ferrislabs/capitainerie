ALTER TABLE agents
    ADD COLUMN config JSONB NOT NULL DEFAULT '{}'
        CHECK (jsonb_typeof(config) = 'object' AND octet_length(config::text) <= 65536),
    ADD COLUMN paused BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN skills TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(skills) <= 200),
    ADD COLUMN revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0);

CREATE TABLE agent_revisions (
    id UUID PRIMARY KEY,
    agent_id UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    revision INTEGER NOT NULL CHECK (revision >= 1),
    snapshot JSONB NOT NULL
        CHECK (jsonb_typeof(snapshot) = 'object' AND octet_length(snapshot::text) <= 262144),
    changed TEXT NOT NULL DEFAULT '' CHECK (char_length(changed) <= 2000),
    author TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT agent_revisions_unique UNIQUE (agent_id, revision)
);

CREATE INDEX agent_revisions_organization ON agent_revisions (organization_id);
