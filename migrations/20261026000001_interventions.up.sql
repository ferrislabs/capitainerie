ALTER TABLE runs
    DROP CONSTRAINT runs_exactly_one_subject,
    ADD CONSTRAINT runs_exactly_one_subject CHECK (
        (transcript_id IS NOT NULL AND card_id IS NULL AND repository IS NULL AND base_branch IS NULL)
        OR (transcript_id IS NULL AND card_id IS NOT NULL AND repository IS NOT NULL AND base_branch IS NOT NULL)
        OR (transcript_id IS NULL AND card_id IS NOT NULL AND repository IS NULL AND base_branch IS NULL)
    );

ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check
        CHECK (kind IN ('extract', 'deduplicate', 'write', 'execute', 'intervene'));

-- The client is the card's at the moment it is handed over, so a later change
-- of the card's client cannot widen what the intervention may reach.
CREATE TABLE interventions (
    run_id UUID PRIMARY KEY REFERENCES runs (id),
    organization_id UUID NOT NULL REFERENCES organizations (id),
    card_id UUID NOT NULL REFERENCES cards (id),
    client_id UUID NOT NULL,
    instructions TEXT NOT NULL DEFAULT '' CHECK (char_length(instructions) <= 4000),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (organization_id, client_id) REFERENCES clients (organization_id, id),
    UNIQUE (run_id, client_id)
);

CREATE TABLE intervention_grants (
    run_id UUID NOT NULL,
    client_id UUID NOT NULL,
    instance_id UUID NOT NULL,
    access TEXT NOT NULL CHECK (access IN ('read', 'write')),
    PRIMARY KEY (run_id, instance_id),
    FOREIGN KEY (run_id, client_id) REFERENCES interventions (run_id, client_id),
    FOREIGN KEY (client_id, instance_id) REFERENCES integration_instances (client_id, id)
);
