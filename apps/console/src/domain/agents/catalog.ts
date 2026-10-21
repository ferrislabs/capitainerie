import type { AgentRole, ModelAssignments } from '#/domain/settings/roles'

export type AgentSource = 'Pipeline' | 'Exécutant' | 'Intervenant' | 'MCP'

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
	'capitainerie-agent-integrator': {
		name: 'Intervenant',
		description:
			'Agit sur les Odoo et n8n d’un client pour une carte confiée, et journalise chaque action.',
		source: 'Intervenant',
		role: 'integrator',
	},
}

export function agentInfo(id: string): AgentInfo | undefined {
	return AGENT_CATALOG[id]
}

export function displayName(id: string): string {
	return agentInfo(id)?.name ?? id
}

const UNLISTED_ROLES: Readonly<Record<string, CatalogRole>> = {
	'capitainerie-agent-captain': 'captain',
}

const WRITER_FALLBACK_ROLES: ReadonlyArray<CatalogRole> = [
	'captain',
	'integrator',
]

export interface AgentModelChoice {
	model: string
	fallback: boolean
}

export function agentRole(id: string): CatalogRole | undefined {
	return agentInfo(id)?.role ?? UNLISTED_ROLES[id]
}

export function agentModelChoice(
	id: string,
	assignments: ModelAssignments | null | undefined,
): AgentModelChoice | undefined {
	const role = agentRole(id)
	if (!role || !assignments) return undefined
	if (role === 'executor') {
		return assignments.executor
			? { model: assignments.executor, fallback: false }
			: undefined
	}
	const own = assignments[role]?.model
	if (own) return { model: own, fallback: false }
	const writer = assignments.writer.model
	return WRITER_FALLBACK_ROLES.includes(role)
		? { model: writer, fallback: true }
		: undefined
}

export function agentModel(
	id: string,
	assignments: ModelAssignments | null | undefined,
): string | undefined {
	return agentModelChoice(id, assignments)?.model
}
