CREATE TABLE transcripts (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    submitted_by TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX transcripts_organization_created ON transcripts (organization_id, created_at DESC);

CREATE TABLE runs (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    transcript_id UUID NOT NULL REFERENCES transcripts (id),
    initiated_by TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    pipeline TEXT[] NOT NULL,
    failure TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE INDEX runs_organization_created ON runs (organization_id, created_at DESC);

-- Written in the transaction that produces the events; a relay publishes them
-- to the bus and stamps `published_at`. Delivery is at least once.
CREATE TABLE outbox_events (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    topic TEXT NOT NULL,
    kind TEXT NOT NULL,
    partition_key UUID NOT NULL,
    envelope JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

CREATE INDEX outbox_events_pending ON outbox_events (id) WHERE published_at IS NULL;
