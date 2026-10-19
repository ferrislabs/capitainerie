ALTER TABLE transcripts
    ADD COLUMN meeting_date DATE,
    ADD COLUMN participants TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN repository TEXT;
