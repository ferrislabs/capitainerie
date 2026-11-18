-- The GitHub issue a worksite card came from. An issue becomes a card once:
-- relaunching it reuses the card.
CREATE TABLE worksite_issues (
    card_id UUID PRIMARY KEY REFERENCES cards (id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    repository TEXT NOT NULL,
    issue_number BIGINT NOT NULL CHECK (issue_number > 0),
    issue_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT worksite_issues_one_card_per_issue UNIQUE (organization_id, repository, issue_number)
);
