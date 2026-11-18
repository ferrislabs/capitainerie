CREATE TABLE approvals (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations (id),
    kind TEXT NOT NULL CHECK (kind IN ('tool_call', 'agent_action')),
    title TEXT NOT NULL CHECK (char_length(title) BETWEEN 1 AND 300),
    detail JSONB NOT NULL DEFAULT '{}'
        CHECK (jsonb_typeof(detail) = 'object' AND octet_length(detail::text) <= 65536),
    requested_by_agent_id UUID REFERENCES agents (id) ON DELETE SET NULL,
    card_id UUID REFERENCES cards (id) ON DELETE SET NULL,
    run_id UUID REFERENCES runs (id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'expired')),
    decided_by TEXT,
    decided_at TIMESTAMPTZ,
    decision_note TEXT CHECK (char_length(decision_note) <= 2000),
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT approvals_decision CHECK (
        CASE status
            WHEN 'pending' THEN decided_by IS NULL AND decided_at IS NULL AND decision_note IS NULL
            WHEN 'expired' THEN decided_by IS NULL
            ELSE decided_by IS NOT NULL AND decided_at IS NOT NULL
        END
    )
);

CREATE INDEX approvals_organization_status ON approvals (organization_id, status, created_at DESC);
