ALTER TABLE transcripts
    DROP CONSTRAINT transcripts_project_keeps_its_client,
    DROP CONSTRAINT transcripts_project_same_organization,
    DROP CONSTRAINT transcripts_client_same_organization,
    DROP COLUMN project_id,
    DROP COLUMN client_id;

ALTER TABLE cards
    DROP CONSTRAINT cards_project_keeps_its_client,
    DROP CONSTRAINT cards_project_same_organization,
    DROP CONSTRAINT cards_client_same_organization,
    DROP COLUMN project_id,
    DROP COLUMN client_id;

DROP TABLE projects;
DROP TABLE clients;
