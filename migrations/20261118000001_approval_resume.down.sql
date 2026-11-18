DROP TABLE approval_grants;
DROP INDEX approvals_pending_tool_call;
ALTER TABLE approvals DROP COLUMN resumed_run_id, DROP COLUMN arguments_hash;
