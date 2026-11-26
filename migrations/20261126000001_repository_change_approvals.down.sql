DROP INDEX approvals_pending_repository_change;
DELETE FROM approvals WHERE kind = 'repository_change';
ALTER TABLE approvals DROP CONSTRAINT approvals_kind_check;
ALTER TABLE approvals ADD CONSTRAINT approvals_kind_check
    CHECK (kind IN ('tool_call', 'agent_action'));
