import type { Schemas } from '#/api/api.client'

export type Column = Schemas.ColumnDto

export const COLUMNS: ReadonlyArray<Column> = [
	'proposed',
	'todo',
	'in_progress',
	'done',
	'rejected',
]

export const COLUMN_LABELS: Record<Column, string> = {
	proposed: 'Proposé',
	todo: 'À faire',
	in_progress: 'En cours',
	done: 'Terminé',
	rejected: 'Rejeté',
}

const AGENT_PREFIX = 'capitainerie-agent-'

export function agentName(clientId: string): string {
	return clientId.startsWith(AGENT_PREFIX)
		? clientId.slice(AGENT_PREFIX.length)
		: clientId
}
