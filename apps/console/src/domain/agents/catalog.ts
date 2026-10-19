import type { AgentRole, ModelAssignments } from '#/domain/settings/roles'

export type AgentSource = 'Pipeline' | 'Exécutant' | 'MCP'

export type CatalogRole = AgentRole | 'executor'

export interface AgentInfo {
	name: string
	description: string
	source: AgentSource
	role: CatalogRole
}

export const AGENT_CATALOG: Readonly<Record<string, AgentInfo>> = {
	'capitainerie-agent-extractor': {
		name: 'Extracteur',
		description: 'Lit le transcript et en extrait les actions à mener.',
		source: 'Pipeline',
		role: 'extractor',
	},
	'capitainerie-agent-judge': {
		name: 'Juge de doublons',
		description: 'Compare chaque action aux cartes existantes.',
		source: 'Pipeline',
		role: 'judge',
	},
	'capitainerie-agent-writer': {
		name: 'Rédacteur',
		description: 'Rédige les cartes proposées et les dépose sur le Kanban.',
		source: 'Pipeline',
		role: 'writer',
	},
	'capitainerie-agent-executor': {
		name: 'Exécutant',
		description: 'Traite une carte confiée sur un dépôt avec Claude Code.',
		source: 'Exécutant',
		role: 'executor',
	},
}

export function agentInfo(id: string): AgentInfo | undefined {
	return AGENT_CATALOG[id]
}

export function displayName(id: string): string {
	return agentInfo(id)?.name ?? id
}

export function agentModel(
	id: string,
	assignments: ModelAssignments | null | undefined,
): string | undefined {
	const role = agentInfo(id)?.role
	if (!role || !assignments) return undefined
	if (role === 'executor') return assignments.executor ?? undefined
	return assignments[role]?.model
}
