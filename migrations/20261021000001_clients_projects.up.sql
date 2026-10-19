CREATE TABLE clients (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    aliases TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(aliases) <= 10),
    notes TEXT NOT NULL DEFAULT '' CHECK (char_length(notes) <= 5000),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ,
    UNIQUE (organization_id, id)
);

CREATE UNIQUE INDEX clients_name_unique ON clients (organization_id, lower(name));

CREATE TABLE projects (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    client_id UUID,
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 120),
    aliases TEXT[] NOT NULL DEFAULT '{}' CHECK (cardinality(aliases) <= 10),
    repository TEXT,
    description TEXT NOT NULL DEFAULT '' CHECK (char_length(description) <= 5000),
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ,
    UNIQUE (organization_id, id),
    UNIQUE (organization_id, id, client_id),
    FOREIGN KEY (organization_id, client_id) REFERENCES clients (organization_id, id)
);

CREATE UNIQUE INDEX projects_name_unique
    ON projects (organization_id, COALESCE(client_id, '00000000-0000-0000-0000-000000000000'::uuid), lower(name));
CREATE INDEX projects_client ON projects (organization_id, client_id);

ALTER TABLE cards
    ADD COLUMN client_id UUID,
    ADD COLUMN project_id UUID,
    ADD CONSTRAINT cards_client_same_organization
        FOREIGN KEY (organization_id, client_id) REFERENCES clients (organization_id, id),
    ADD CONSTRAINT cards_project_same_organization
        FOREIGN KEY (organization_id, project_id) REFERENCES projects (organization_id, id),
    ADD CONSTRAINT cards_project_keeps_its_client
        FOREIGN KEY (organization_id, project_id, client_id)
        REFERENCES projects (organization_id, id, client_id) ON UPDATE CASCADE;

CREATE INDEX cards_client ON cards (organization_id, client_id) WHERE client_id IS NOT NULL;
CREATE INDEX cards_project ON cards (organization_id, project_id) WHERE project_id IS NOT NULL;

ALTER TABLE transcripts
    ADD COLUMN client_id UUID,
    ADD COLUMN project_id UUID,
    ADD CONSTRAINT transcripts_client_same_organization
        FOREIGN KEY (organization_id, client_id) REFERENCES clients (organization_id, id),
    ADD CONSTRAINT transcripts_project_same_organization
        FOREIGN KEY (organization_id, project_id) REFERENCES projects (organization_id, id),
    ADD CONSTRAINT transcripts_project_keeps_its_client
        FOREIGN KEY (organization_id, project_id, client_id)
        REFERENCES projects (organization_id, id, client_id) ON UPDATE CASCADE;

CREATE INDEX transcripts_client ON transcripts (organization_id, client_id) WHERE client_id IS NOT NULL;
CREATE INDEX transcripts_project ON transcripts (organization_id, project_id) WHERE project_id IS NOT NULL;
