DROP TABLE agent_tasks;

ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check
        CHECK (kind IN ('extract', 'deduplicate', 'write', 'execute', 'intervene')) NOT VALID;
