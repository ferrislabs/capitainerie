const KIND_LABELS: Record<string, string> = {
	extract: 'Extraction',
	deduplicate: 'Dédoublonnage',
	write: 'Rédaction',
	execute: 'Exécution',
}

const AGENT_PREFIX = 'capitainerie-agent-'

export function stepKindLabel(kind: string): string {
	return KIND_LABELS[kind] ?? kind
}

export function agentName(agent: string): string {
	return agent.startsWith(AGENT_PREFIX)
		? agent.slice(AGENT_PREFIX.length)
		: agent
}
