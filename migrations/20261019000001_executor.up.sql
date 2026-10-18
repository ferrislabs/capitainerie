ALTER TABLE runs
    ALTER COLUMN transcript_id DROP NOT NULL,
    ADD COLUMN card_id UUID REFERENCES cards (id),
    ADD COLUMN repository TEXT,
    ADD COLUMN base_branch TEXT,
    ADD CONSTRAINT runs_exactly_one_subject CHECK (
        (transcript_id IS NOT NULL AND card_id IS NULL AND repository IS NULL AND base_branch IS NULL)
        OR (transcript_id IS NULL AND card_id IS NOT NULL AND repository IS NOT NULL AND base_branch IS NOT NULL)
    );

CREATE UNIQUE INDEX runs_one_active_execution
    ON runs (card_id)
    WHERE card_id IS NOT NULL AND status IN ('queued', 'running');

ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check CHECK (kind IN ('extract', 'deduplicate', 'write', 'execute'));

-- One GitHub App installation per organization.
CREATE TABLE github_installations (
    organization_id UUID PRIMARY KEY REFERENCES organizations (id),
    installation_id BIGINT NOT NULL,
    account_login TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

-- Per-execution credentials of the egress proxy; only the SHA-256 of the token is kept.
CREATE TABLE task_grants (
    token_sha256 BYTEA PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES runs (id) ON DELETE CASCADE,
    organization_id UUID NOT NULL,
    repository TEXT NOT NULL,
    branch TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ
);

CREATE INDEX task_grants_run ON task_grants (run_id);
