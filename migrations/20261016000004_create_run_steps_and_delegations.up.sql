ALTER TABLE runs
    ADD COLUMN heartbeat_at TIMESTAMPTZ,
    ADD COLUMN trace_id TEXT;

-- One row per pipeline step; a resumed run skips the steps already succeeded.
CREATE TABLE run_steps (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES runs (id),
    organization_id UUID NOT NULL REFERENCES organizations (id),
    seq INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('extract', 'deduplicate', 'write')),
    agent TEXT,
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed', 'skipped')),
    attempt INTEGER NOT NULL,
    output JSONB,
    model TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_micros BIGINT NOT NULL DEFAULT 0,
    latency_ms BIGINT,
    error TEXT,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ,
    UNIQUE (run_id, seq)
);

-- Interim (docs/ferriskey-needs.md, need 2): the agent-addressed token the
-- API obtained at submission, AES-256-GCM encrypted, purged when the run ends.
CREATE TABLE run_delegations (
    run_id UUID NOT NULL REFERENCES runs (id),
    agent_client_id TEXT NOT NULL,
    nonce BYTEA NOT NULL,
    ciphertext BYTEA NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (run_id, agent_client_id)
);
