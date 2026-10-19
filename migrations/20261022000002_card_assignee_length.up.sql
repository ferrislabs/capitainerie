ALTER TABLE cards
    ADD CONSTRAINT cards_assignee_subject_length CHECK (char_length(assignee_subject) <= 255);
