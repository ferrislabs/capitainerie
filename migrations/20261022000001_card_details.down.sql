DROP TABLE members;

DROP INDEX cards_priority;
DROP INDEX cards_assignee;

ALTER TABLE cards
    DROP COLUMN estimate,
    DROP COLUMN labels,
    DROP COLUMN assignee_subject,
    DROP COLUMN due_date,
    DROP COLUMN priority,
    DROP COLUMN acceptance_criteria;
