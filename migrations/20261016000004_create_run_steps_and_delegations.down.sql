DROP TABLE run_delegations;
DROP TABLE run_steps;
ALTER TABLE runs DROP COLUMN trace_id, DROP COLUMN heartbeat_at;
