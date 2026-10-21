ALTER TABLE integration_actions
    DROP COLUMN revert_claimed_at,
    DROP CONSTRAINT integration_actions_instance_same_organization;

ALTER TABLE integration_instances
    DROP CONSTRAINT integration_instances_organization_id_id_key;
