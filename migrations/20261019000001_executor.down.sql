DROP TABLE task_grants;
DROP TABLE github_installations;

DELETE FROM run_steps WHERE kind = 'execute';
ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check CHECK (kind IN ('extract', 'deduplicate', 'write'));

DROP INDEX runs_one_active_execution;

DELETE FROM run_delegations WHERE run_id IN (SELECT id FROM runs WHERE card_id IS NOT NULL);
DELETE FROM runs WHERE card_id IS NOT NULL;
ALTER TABLE runs
    DROP CONSTRAINT runs_exactly_one_subject,
    DROP COLUMN base_branch,
    DROP COLUMN repository,
    DROP COLUMN card_id,
    ALTER COLUMN transcript_id SET NOT NULL;
