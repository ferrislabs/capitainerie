CREATE TABLE skills (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    slug TEXT NOT NULL CHECK (char_length(slug) BETWEEN 1 AND 60),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    description TEXT NOT NULL DEFAULT '' CHECK (char_length(description) <= 5000),
    content TEXT NOT NULL DEFAULT '' CHECK (char_length(content) <= 100000),
    source TEXT NOT NULL DEFAULT 'manual' CHECK (char_length(source) BETWEEN 1 AND 2000),
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT skills_slug_unique UNIQUE (organization_id, slug)
);

CREATE INDEX agents_skills ON agents USING GIN (skills);
