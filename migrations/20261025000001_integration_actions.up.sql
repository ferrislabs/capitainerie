CREATE TABLE integration_actions (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    run_id UUID REFERENCES runs (id),
    card_id UUID REFERENCES cards (id),
    instance_id UUID NOT NULL REFERENCES integration_instances (id),
    subject TEXT NOT NULL,
    actor TEXT NOT NULL,
    tool TEXT NOT NULL CHECK (char_length(tool) BETWEEN 1 AND 64),
    access TEXT NOT NULL CHECK (access IN ('read', 'write')),
    target JSONB NOT NULL,
    arguments JSONB NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('pending', 'succeeded', 'denied', 'failed')),
    reason_code TEXT,
    reason TEXT CHECK (char_length(reason) <= 2000),
    state_before JSONB,
    state_after JSONB,
    snapshot_nonce BYTEA,
    snapshot_ciphertext BYTEA,
    revertible BOOLEAN NOT NULL DEFAULT false,
    reverts_action_id UUID REFERENCES integration_actions (id),
    reverted_by_action_id UUID REFERENCES integration_actions (id),
    latency_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    CONSTRAINT integration_actions_snapshot_whole
        CHECK ((snapshot_nonce IS NULL) = (snapshot_ciphertext IS NULL)),
    CONSTRAINT integration_actions_pending_is_a_write
        CHECK (outcome <> 'pending' OR access = 'write')
);

CREATE INDEX integration_actions_run
    ON integration_actions (organization_id, run_id, created_at)
    WHERE run_id IS NOT NULL;

CREATE INDEX integration_actions_card
    ON integration_actions (organization_id, card_id, created_at)
    WHERE card_id IS NOT NULL;
