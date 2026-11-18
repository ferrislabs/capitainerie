import type { Schemas } from '#/api/api.client'

export const AGENT_LEAD = 'a0000000-0000-4000-8000-000000000001'
export const AGENT_SUB = 'a0000000-0000-4000-8000-000000000002'

export const agent = (
	id: string,
	name: string,
	extra: Partial<Schemas.TeamAgentResponse> = {},
): Schemas.TeamAgentResponse => ({
	id,
	name,
	slug: name.toLowerCase(),
	role: 'Rôle',
	description: '',
	instructions: '',
	model: 'claude-test',
	hue: 120,
	status: 'idle',
	archived: false,
	connector_grants: [],
	created_at: '2026-11-01T00:00:00Z',
	updated_at: '2026-11-01T00:00:00Z',
	created_by: 'u',
	...extra,
})

export const card = (
	id: string,
	extra: Partial<Schemas.CardResponse> = {},
): Schemas.CardResponse => ({
	id,
	column: 'todo',
	title: `Carte ${id.slice(0, 4)}`,
	description: '',
	labels: [],
	acceptance_criteria: [],
	priority: 'normal',
	created_at: new Date().toISOString(),
	updated_at: new Date().toISOString(),
	created_by_subject: 'u',
	version: 3,
	...extra,
})

export const AGENTS_ROUTE = {
	'GET /api/v1/team/agents': {
		items: [
			agent(AGENT_LEAD, 'Veille légale (lead)'),
			agent(AGENT_SUB, 'Registre', { reports_to: AGENT_LEAD }),
		],
	},
}

export const jsonBody = (
	fetchMock: { mock: { calls: ReadonlyArray<unknown[]> } },
	method: string,
	path: string,
): unknown => {
	const call = fetchMock.mock.calls.find(([input, init]) => {
		const url = new URL(String(input))
		return (
			((init as RequestInit | undefined)?.method ?? 'GET') === method &&
			url.pathname === path
		)
	})
	const body = (call?.[1] as RequestInit | undefined)?.body
	return typeof body === 'string' ? JSON.parse(body) : undefined
}
