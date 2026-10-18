-- Models frozen when the run is queued (agent role -> provider and model).
ALTER TABLE runs ADD COLUMN models JSONB;
