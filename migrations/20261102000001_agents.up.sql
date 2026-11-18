CREATE TABLE agents (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    slug TEXT NOT NULL CHECK (char_length(slug) BETWEEN 1 AND 60),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    role TEXT NOT NULL DEFAULT '' CHECK (char_length(role) <= 120),
    description TEXT NOT NULL DEFAULT '' CHECK (char_length(description) <= 5000),
    model TEXT NOT NULL DEFAULT '' CHECK (char_length(model) <= 200),
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'running')),
    hue SMALLINT NOT NULL DEFAULT 0 CHECK (hue BETWEEN 0 AND 360),
    reports_to UUID REFERENCES agents (id) ON DELETE SET NULL,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    instructions TEXT NOT NULL DEFAULT '' CHECK (char_length(instructions) <= 20000),
    connector_grants TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(connector_grants) <= 50),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT agents_slug_unique UNIQUE (organization_id, slug)
);

CREATE INDEX agents_reports_to ON agents (reports_to) WHERE reports_to IS NOT NULL;
