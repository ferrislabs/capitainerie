DROP TABLE agent_revisions;

ALTER TABLE agents
    DROP COLUMN revision,
    DROP COLUMN skills,
    DROP COLUMN paused,
    DROP COLUMN config;
