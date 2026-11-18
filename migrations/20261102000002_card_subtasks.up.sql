ALTER TABLE cards
    ADD COLUMN parent_id UUID REFERENCES cards (id) ON DELETE SET NULL,
    ADD COLUMN assignee_agent_id UUID REFERENCES agents (id) ON DELETE SET NULL,
    ADD CONSTRAINT cards_not_its_own_parent CHECK (parent_id IS DISTINCT FROM id);

CREATE INDEX cards_parent ON cards (organization_id, parent_id) WHERE parent_id IS NOT NULL;
CREATE INDEX cards_assignee_agent ON cards (organization_id, assignee_agent_id) WHERE assignee_agent_id IS NOT NULL;
