import type { Schemas } from '#/api/api.client'
import { canDrop } from '#/domain/kanban/can-drop'
import type { Column } from '#/domain/kanban/columns'
import { columnOfData } from '#/domain/kanban/drag-data'

export interface Drop {
	cardId: string
	to: Column
	expectedVersion: number
}

export function resolveDrop(
	cards: ReadonlyArray<Schemas.CardResponse>,
	activeId: string | number,
	overData: unknown,
): Drop | undefined {
	const card = cards.find((candidate) => candidate.id === activeId)
	const to = columnOfData(overData)
	if (!card || !to || !canDrop(card.column, to)) return undefined
	return { cardId: card.id, to, expectedVersion: card.version }
}
