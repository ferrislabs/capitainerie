ALTER TABLE integration_instances
    ADD CONSTRAINT integration_instances_organization_id_id_key UNIQUE (organization_id, id);

ALTER TABLE integration_actions
    ADD CONSTRAINT integration_actions_instance_same_organization
        FOREIGN KEY (organization_id, instance_id)
        REFERENCES integration_instances (organization_id, id),
    ADD COLUMN revert_claimed_at TIMESTAMPTZ;
