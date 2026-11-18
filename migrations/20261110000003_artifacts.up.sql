-- A deliverable an agent produced: a text kept here, or a link to it.
CREATE TABLE artifacts (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    card_id UUID REFERENCES cards (id) ON DELETE SET NULL,
    run_id UUID REFERENCES runs (id) ON DELETE SET NULL,
    agent_id UUID REFERENCES agents (id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('markdown', 'json', 'link')),
    title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 300),
    content TEXT CHECK (octet_length(content) <= 1048576),
    url TEXT CHECK (char_length(url) <= 2000),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK ((kind = 'link') = (url IS NOT NULL AND content IS NULL)),
    CHECK (kind = 'link' OR content IS NOT NULL)
);

CREATE INDEX artifacts_organization ON artifacts (organization_id, id DESC);
CREATE INDEX artifacts_card ON artifacts (card_id) WHERE card_id IS NOT NULL;
CREATE INDEX artifacts_agent ON artifacts (organization_id, agent_id) WHERE agent_id IS NOT NULL;
-- One report per agent run.
CREATE UNIQUE INDEX artifacts_run_report ON artifacts (run_id) WHERE run_id IS NOT NULL;
