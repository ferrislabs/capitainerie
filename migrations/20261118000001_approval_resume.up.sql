ALTER TABLE approvals
    ADD COLUMN arguments_hash TEXT CHECK (arguments_hash ~ '^[0-9a-f]{64}$'),
    ADD COLUMN resumed_run_id UUID REFERENCES runs (id) ON DELETE SET NULL;

CREATE UNIQUE INDEX approvals_pending_tool_call
    ON approvals (card_id, (detail ->> 'tool'), arguments_hash)
    WHERE status = 'pending' AND arguments_hash IS NOT NULL;

CREATE TABLE approval_grants (
    approval_id UUID PRIMARY KEY REFERENCES approvals (id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    run_id UUID NOT NULL REFERENCES runs (id) ON DELETE CASCADE,
    tool TEXT NOT NULL CHECK (char_length(tool) BETWEEN 1 AND 64),
    arguments_hash TEXT NOT NULL CHECK (arguments_hash ~ '^[0-9a-f]{64}$'),
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX approval_grants_run ON approval_grants (run_id);
