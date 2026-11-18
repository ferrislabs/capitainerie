import type { Approval } from '#/domain/approvals/model'

export function approval(id: string, extra: Partial<Approval> = {}): Approval {
	return {
		id,
		kind: 'tool_call',
		title: `Demande ${id}`,
		detail: { tool: 'gmail.send', arguments: { to: 'client@example.com' } },
		requested_by_agent_id: null,
		card_id: null,
		run_id: null,
		status: 'pending',
		decided_by: null,
		decided_at: null,
		decision_note: null,
		created_at: '2026-11-18T09:00:00Z',
		resumed_run_id: null,
		...extra,
	}
}
