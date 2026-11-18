export const AGENT_FILTERS = [
	{ id: 'tous', label: 'Tous' },
	{ id: 'actifs', label: 'Actifs' },
	{ id: 'pause', label: 'En pause' },
	{ id: 'erreur', label: 'Erreur' },
	{ id: 'archives', label: 'Archivés' },
] as const

export type AgentFilter = (typeof AGENT_FILTERS)[number]['id']

export function parseAgentFilter(value: unknown): AgentFilter {
	return AGENT_FILTERS.find((f) => f.id === value)?.id ?? 'tous'
}
