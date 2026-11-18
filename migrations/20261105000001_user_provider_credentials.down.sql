ALTER TABLE agent_tasks
    DROP COLUMN model,
    DROP COLUMN provider;

DROP TABLE user_provider_credentials;
