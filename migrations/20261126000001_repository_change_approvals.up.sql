ALTER TABLE approvals DROP CONSTRAINT approvals_kind_check;
ALTER TABLE approvals ADD CONSTRAINT approvals_kind_check
    CHECK (kind IN ('tool_call', 'agent_action', 'repository_change'));

-- A card waits on one plan at a time: a newer plan expires the older one.
CREATE UNIQUE INDEX approvals_pending_repository_change
    ON approvals (card_id)
    WHERE status = 'pending' AND kind = 'repository_change';
