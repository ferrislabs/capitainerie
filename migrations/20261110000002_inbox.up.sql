-- One notification for one member, projected from an outbox event. The
-- projection is replayable: an event gives a member at most one item.
CREATE TABLE inbox_items (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    member_subject TEXT NOT NULL CHECK (char_length(member_subject) BETWEEN 1 AND 255),
    event_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('approval', 'run', 'card', 'source', 'routine')),
    title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 300),
    body TEXT NOT NULL DEFAULT '' CHECK (char_length(body) <= 1000),
    link TEXT CHECK (char_length(link) <= 500),
    card_id UUID,
    run_id UUID,
    created_at TIMESTAMPTZ NOT NULL,
    read_at TIMESTAMPTZ,
    CONSTRAINT inbox_items_event_unique UNIQUE (organization_id, member_subject, event_id)
);

CREATE INDEX inbox_items_member ON inbox_items (organization_id, member_subject, id DESC);
CREATE INDEX inbox_items_unread ON inbox_items (organization_id, member_subject)
    WHERE read_at IS NULL;

-- How far a projection read the outbox. Events are read in id order (uuid v7).
CREATE TABLE projection_cursors (
    name TEXT PRIMARY KEY,
    last_event_id UUID NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- The inbox starts with the events to come, not the whole history.
INSERT INTO projection_cursors (name, last_event_id, updated_at)
VALUES (
    'inbox',
    COALESCE(
        (SELECT id FROM outbox_events ORDER BY id DESC LIMIT 1),
        '00000000-0000-0000-0000-000000000000'
    ),
    now()
);
