import type { Schemas } from '#/api/api.client'

export function teamAgent(
	id: string,
	name: string,
	extra: Partial<Schemas.TeamAgentResponse> = {},
): Schemas.TeamAgentResponse {
	return {
		id,
		slug: id,
		name,
		role: 'PM',
		description: `Description ${name}`,
		model: 'Claude Sonnet 5.5',
		status: 'idle',
		hue: 120,
		reports_to: null,
		archived: false,
		instructions: `# ${name}`,
		connector_grants: [],
		config: {},
		skills: [],
		paused: false,
		revision: 1,
		created_by: 'u',
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}

export const CHIEF = teamAgent('a-chief', 'Chief of Staff', { role: 'CEO' })
export const LEGAL = teamAgent('a-legal', 'Veille légale (Sous-domaines)', {
	status: 'running',
	reports_to: 'a-chief',
})
export const REGISTRE = teamAgent('a-registre', 'Registre (RNE & BODACC)', {
	reports_to: 'a-legal',
})
export const DEV = teamAgent('a-dev', 'Dev (cycle autonome)', {
	role: 'Ingénieur',
	reports_to: 'a-chief',
	instructions: '# Dev · Natalia',
})

export const TEAM = [CHIEF, LEGAL, REGISTRE, DEV]

export function teamRoutes(
	agent: Schemas.TeamAgentResponse = DEV,
	items: ReadonlyArray<Schemas.TeamAgentResponse> = TEAM,
) {
	return {
		'GET /api/v1/team/agents': { items },
		[`GET /api/v1/team/agents/${agent.id}`]: agent,
	}
}
