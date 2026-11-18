export const AGENT_TABS = [
	{ id: 'overview', label: "Vue d'ensemble" },
	{ id: 'instructions', label: 'Instructions' },
	{ id: 'skills', label: 'Compétences' },
	{ id: 'sources', label: 'Sources' },
	{ id: 'team', label: 'Délégation' },
	{ id: 'connectors', label: 'Connecteurs' },
	{ id: 'runtime', label: 'Runtime' },
	{ id: 'perms', label: 'Permissions' },
	{ id: 'rev', label: 'Révisions' },
] as const

export type AgentTabId = (typeof AGENT_TABS)[number]['id']

const LEGACY: Readonly<Record<string, AgentTabId>> = {
	tools: 'connectors',
	channels: 'connectors',
}

export function resolveAgentTab(segment: string | undefined): AgentTabId {
	const known = AGENT_TABS.find((tab) => tab.id === segment)
	if (known) return known.id
	return (segment ? LEGACY[segment] : undefined) ?? 'overview'
}
