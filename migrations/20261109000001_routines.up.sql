CREATE TABLE routines (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    slug TEXT NOT NULL CHECK (char_length(slug) BETWEEN 1 AND 60),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    -- The task description of every card the routine creates.
    prompt TEXT NOT NULL CHECK (char_length(prompt) BETWEEN 1 AND 20000),
    agent_id UUID NOT NULL REFERENCES agents (id),
    project_id UUID,
    priority TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    -- `{nom}` and `{date}` are replaced when the card is created.
    title_template TEXT NOT NULL CHECK (char_length(title_template) BETWEEN 1 AND 200),
    timezone TEXT NOT NULL DEFAULT 'Europe/Paris' CHECK (char_length(timezone) BETWEEN 1 AND 64),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    -- When the previous card is still open: skip the passage, queue it until
    -- that card closes, or create a card anyway.
    if_open TEXT NOT NULL CHECK (if_open IN ('skip', 'queue', 'always')),
    -- When a planned passage was missed (worker down): skip it, run it once,
    -- or run every missed passage.
    if_missed TEXT NOT NULL CHECK (if_missed IN ('skip', 'run_once', 'run_all')),
    allow_writes BOOLEAN NOT NULL DEFAULT FALSE,
    last_run_at TIMESTAMPTZ,
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT routines_slug_unique UNIQUE (organization_id, slug),
    CONSTRAINT routines_project_same_organization
        FOREIGN KEY (organization_id, project_id) REFERENCES projects (organization_id, id)
);

-- What starts a routine: a planning (its schedule and next passage) or a
-- webhook (only the hash of its token is kept).
CREATE TABLE routine_triggers (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    routine_id UUID NOT NULL REFERENCES routines (id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('planning', 'webhook')),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    frequency TEXT CHECK (frequency IN ('every_hours', 'daily', 'weekly', 'monthly', 'cron')),
    every_hours SMALLINT CHECK (every_hours BETWEEN 1 AND 744),
    at_time TEXT CHECK (at_time ~ '^[0-2][0-9]:[0-5][0-9]$'),
    weekdays TEXT[] NOT NULL DEFAULT '{}',
    cron TEXT CHECK (char_length(cron) BETWEEN 1 AND 120),
    next_run_at TIMESTAMPTZ,
    webhook_token_hash TEXT CHECK (char_length(webhook_token_hash) = 64),
    webhook_token_hint TEXT CHECK (char_length(webhook_token_hint) <= 8),
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT routine_triggers_webhook_token_unique UNIQUE (webhook_token_hash),
    CHECK (
        (kind = 'planning' AND frequency IS NOT NULL AND webhook_token_hash IS NULL)
        OR (kind = 'webhook' AND frequency IS NULL AND next_run_at IS NULL
            AND webhook_token_hash IS NOT NULL AND webhook_token_hint IS NOT NULL)
    )
);

CREATE INDEX routine_triggers_routine ON routine_triggers (routine_id, created_at, id);
CREATE INDEX routine_triggers_due ON routine_triggers (next_run_at) WHERE enabled AND next_run_at IS NOT NULL;

-- One passage of a routine. `pending` while its card is being created,
-- `queued` while the previous card is still open.
CREATE TABLE routine_runs (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    routine_id UUID NOT NULL REFERENCES routines (id) ON DELETE CASCADE,
    trigger_id UUID REFERENCES routine_triggers (id) ON DELETE SET NULL,
    origin TEXT NOT NULL CHECK (origin IN ('planning', 'manual', 'webhook')),
    scheduled_for TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'queued', 'created', 'skipped', 'failed')),
    card_id UUID REFERENCES cards (id) ON DELETE SET NULL,
    run_id UUID REFERENCES runs (id) ON DELETE SET NULL,
    reason TEXT CHECK (char_length(reason) <= 2000)
);

CREATE INDEX routine_runs_routine_started ON routine_runs (routine_id, started_at DESC, id DESC);
CREATE INDEX routine_runs_open ON routine_runs (trigger_id) WHERE status IN ('pending', 'queued');
