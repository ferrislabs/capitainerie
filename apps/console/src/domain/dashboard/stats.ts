import type { Schemas } from '#/api/api.client'

export interface CardStats {
	open: number
	inProgress: number
	proposed: number
}

export function cardStats(
	cards: ReadonlyArray<Schemas.CardResponse>,
): CardStats {
	const count = (column: Schemas.ColumnDto) =>
		cards.filter((card) => card.column === column).length
	const todo = count('todo')
	const inProgress = count('in_progress')
	return { open: todo + inProgress, inProgress, proposed: count('proposed') }
}

export function activeAgentCount(
	active: ReadonlyArray<Schemas.ActiveRunResponse>,
): number {
	const names = active.flatMap((item) => item.current_step?.agent ?? [])
	return new Set(names).size
}

export function recentCards(
	cards: ReadonlyArray<Schemas.CardResponse>,
	limit: number,
): ReadonlyArray<Schemas.CardResponse> {
	return [...cards]
		.filter((card) => card.column !== 'rejected')
		.sort((a, b) => b.updated_at.localeCompare(a.updated_at))
		.slice(0, limit)
}

export type Severity = 'crit' | 'warn' | 'info'
export type AttentionTarget = 'runs' | 'tasks'

export interface AttentionItem {
	severity: Severity
	title: string
	detail: string
	action: string
	target: AttentionTarget
}

export function attentionItems(
	cards: ReadonlyArray<Schemas.CardResponse>,
	runs: ReadonlyArray<Schemas.RunResponse>,
): ReadonlyArray<AttentionItem> {
	const failed = runs.filter((run) => run.status === 'failed')
	const proposed = cards.filter((card) => card.column === 'proposed')
	const items: AttentionItem[] = []
	const firstFailed = failed[0]
	if (firstFailed) {
		items.push({
			severity: 'crit',
			title: `${failed.length} ${failed.length > 1 ? 'exécutions récentes en échec' : 'exécution récente en échec'}`,
			detail: firstFailed.failure ?? firstFailed.title ?? 'Échec sans détail',
			action: 'Voir les exécutions',
			target: 'runs',
		})
	}
	const firstProposed = proposed[0]
	if (firstProposed) {
		items.push({
			severity: 'warn',
			title: `${proposed.length} ${proposed.length > 1 ? 'tâches proposées à valider' : 'tâche proposée à valider'}`,
			detail: firstProposed.title,
			action: 'Voir les tâches',
			target: 'tasks',
		})
	}
	return items
}
