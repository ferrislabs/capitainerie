-- The audit log reads the outbox of an organization, newest first.
CREATE INDEX outbox_events_organization ON outbox_events (organization_id, id DESC);

-- A monthly spending limit: one for the organization (agent_id NULL), and at
-- most one per agent.
CREATE TABLE budgets (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    agent_id UUID REFERENCES agents (id) ON DELETE CASCADE,
    monthly_limit_cents BIGINT NOT NULL CHECK (monthly_limit_cents BETWEEN 1 AND 100000000000),
    alert_threshold_percent SMALLINT NOT NULL DEFAULT 80
        CHECK (alert_threshold_percent BETWEEN 1 AND 100),
    updated_by TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX budgets_organization_scope
    ON budgets (organization_id) WHERE agent_id IS NULL;
CREATE UNIQUE INDEX budgets_agent_scope
    ON budgets (organization_id, agent_id) WHERE agent_id IS NOT NULL;

-- The month's spend sums the steps started since the first of the month.
CREATE INDEX run_steps_organization_started ON run_steps (organization_id, started_at);
