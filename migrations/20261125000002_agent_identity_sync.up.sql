-- False while FerrisKey still has to follow an archive or an unarchive.
ALTER TABLE agent_identities ADD COLUMN synced BOOLEAN NOT NULL DEFAULT true;

CREATE INDEX agent_identities_unsynced ON agent_identities (updated_at) WHERE NOT synced;
