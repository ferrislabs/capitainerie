ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check
        CHECK (kind IN ('extract', 'deduplicate', 'write', 'execute', 'intervene', 'agent_task'));

CREATE TABLE agent_tasks (
    run_id UUID PRIMARY KEY REFERENCES runs (id),
    organization_id UUID NOT NULL REFERENCES organizations (id),
    card_id UUID NOT NULL REFERENCES cards (id),
    agent_id UUID NOT NULL REFERENCES agents (id),
    allow_writes BOOLEAN NOT NULL,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX agent_tasks_card ON agent_tasks (card_id);
