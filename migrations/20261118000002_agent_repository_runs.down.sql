DROP TABLE executor_status;
ALTER TABLE task_grants DROP COLUMN initiated_by, DROP COLUMN mode;
ALTER TABLE agents DROP COLUMN repository_grants;
