DROP TABLE intervention_grants;
DROP TABLE interventions;

ALTER TABLE run_steps
    DROP CONSTRAINT run_steps_kind_check,
    ADD CONSTRAINT run_steps_kind_check
        CHECK (kind IN ('extract', 'deduplicate', 'write', 'execute')) NOT VALID;

ALTER TABLE runs
    DROP CONSTRAINT runs_exactly_one_subject,
    ADD CONSTRAINT runs_exactly_one_subject CHECK (
        (transcript_id IS NOT NULL AND card_id IS NULL AND repository IS NULL AND base_branch IS NULL)
        OR (transcript_id IS NULL AND card_id IS NOT NULL AND repository IS NOT NULL AND base_branch IS NOT NULL)
    ) NOT VALID;
