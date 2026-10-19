ALTER TABLE cards
    ADD COLUMN acceptance_criteria JSONB NOT NULL DEFAULT '[]'
        CHECK (jsonb_typeof(acceptance_criteria) = 'array' AND jsonb_array_length(acceptance_criteria) <= 20),
    ADD COLUMN priority TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    ADD COLUMN due_date DATE,
    ADD COLUMN assignee_subject TEXT,
    ADD COLUMN labels TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(labels) <= 10),
    ADD COLUMN estimate TEXT CHECK (estimate IN ('xs', 's', 'm', 'l', 'xl'));

CREATE INDEX cards_assignee ON cards (organization_id, assignee_subject) WHERE assignee_subject IS NOT NULL;
CREATE INDEX cards_priority ON cards (organization_id, priority) WHERE priority <> 'normal';

CREATE TABLE members (
    organization_id UUID NOT NULL REFERENCES organizations (id),
    subject TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    last_seen_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (organization_id, subject)
);
