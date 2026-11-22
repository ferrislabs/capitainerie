ALTER TABLE provider_credentials
    DROP CONSTRAINT provider_credentials_kind_check,
    ADD CONSTRAINT provider_credentials_kind_check
        CHECK (kind IN ('anthropic', 'mistral', 'openai', 'google', 'openai_compatible'));

ALTER TABLE user_provider_credentials
    DROP CONSTRAINT user_provider_credentials_kind_check,
    ADD CONSTRAINT user_provider_credentials_kind_check
        CHECK (kind IN ('anthropic', 'mistral', 'openai', 'google', 'openai_compatible'));

ALTER TABLE agent_tasks
    DROP CONSTRAINT agent_tasks_provider_check,
    ADD CONSTRAINT agent_tasks_provider_check
        CHECK (provider IN ('anthropic', 'mistral', 'openai', 'google', 'openai_compatible'));
