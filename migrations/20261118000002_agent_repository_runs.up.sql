ALTER TABLE agents
    ADD COLUMN repository_grants TEXT[] NOT NULL DEFAULT '{}';

ALTER TABLE task_grants
    ADD COLUMN mode TEXT NOT NULL DEFAULT 'change' CHECK (mode IN ('change', 'audit')),
    ADD COLUMN initiated_by TEXT;

CREATE TABLE executor_status (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    enabled BOOLEAN NOT NULL,
    reason TEXT,
    updated_at TIMESTAMPTZ NOT NULL
);
