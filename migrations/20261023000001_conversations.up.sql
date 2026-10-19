-- The chat with the Captain. A conversation belongs to the member who opened
-- it, inside their organization; nobody else reads it.
-- `turn_started_at` is the lease of the single turn running in a conversation:
-- NULL when idle, and ignored once older than the lease so a crashed process
-- never locks a conversation for good.
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    owner_subject TEXT NOT NULL,
    title TEXT NOT NULL,
    model JSONB,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_micros BIGINT NOT NULL DEFAULT 0,
    turn_started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ
);

CREATE INDEX conversations_owner
    ON conversations (organization_id, owner_subject, updated_at DESC)
    WHERE archived_at IS NULL;

CREATE TABLE conversation_messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'tool')),
    content JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('complete', 'interrupted', 'failed')),
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE (conversation_id, seq)
);
