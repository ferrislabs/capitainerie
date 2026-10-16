CREATE TABLE cards (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    "column" TEXT NOT NULL CHECK ("column" IN ('proposed', 'todo', 'in_progress', 'done', 'rejected')),
    -- The exact passage of the transcript the card comes from, as character
    -- offsets [start, end) into transcripts.body plus the quoted text.
    provenance_transcript_id UUID REFERENCES transcripts (id),
    provenance_start INTEGER,
    provenance_end INTEGER,
    provenance_quote TEXT,
    created_by_run UUID REFERENCES runs (id),
    created_by_subject TEXT NOT NULL,
    created_by_actor TEXT,
    version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK (
        (provenance_transcript_id IS NULL AND provenance_start IS NULL AND provenance_end IS NULL AND provenance_quote IS NULL)
        OR (provenance_transcript_id IS NOT NULL AND provenance_start >= 0 AND provenance_end > provenance_start AND provenance_quote IS NOT NULL)
    )
);

CREATE INDEX cards_organization_column ON cards (organization_id, "column", updated_at DESC);

CREATE TABLE card_comments (
    id UUID PRIMARY KEY,
    card_id UUID NOT NULL REFERENCES cards (id),
    organization_id UUID NOT NULL REFERENCES organizations (id),
    author_subject TEXT NOT NULL,
    author_actor TEXT,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX card_comments_card ON card_comments (card_id, created_at);

-- Every decision of the policy decision point, allowed or denied. Written
-- outside the business transaction so a denial is kept even when the
-- transaction rolls back.
CREATE TABLE authorization_decisions (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    subject TEXT NOT NULL,
    actor TEXT,
    action TEXT NOT NULL,
    resource_kind TEXT NOT NULL,
    resource_id UUID,
    allowed BOOLEAN NOT NULL,
    reason TEXT,
    run_id UUID,
    decided_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX authorization_decisions_run ON authorization_decisions (run_id, decided_at) WHERE run_id IS NOT NULL;
CREATE INDEX authorization_decisions_organization ON authorization_decisions (organization_id, decided_at DESC);
