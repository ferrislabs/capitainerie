import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { resolveDrop } from '#/domain/kanban/drop'

function card(
	id: string,
	column: Schemas.ColumnDto,
	version = 1,
): Schemas.CardResponse {
	return {
		id,
		title: id,
		description: '',
		column,
		provenance: null,
		created_by_run: null,
		created_by_subject: 'ada',
		created_by_agent: null,
		version,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
		acceptance_criteria: [],
		priority: 'normal',
		labels: [],
	}
}

describe('resolveDrop', () => {
	const cards = [card('a', 'todo', 7), card('b', 'done')]

	it('moves a card to the column under it with its version', () => {
		expect(resolveDrop(cards, 'a', { column: 'in_progress' })).toEqual({
			cardId: 'a',
			to: 'in_progress',
			expectedVersion: 7,
		})
	})

	it('does nothing when dropped on its own column', () => {
		expect(resolveDrop(cards, 'a', { column: 'todo' })).toBeUndefined()
	})

	it('does nothing outside any column or for an unknown card', () => {
		expect(resolveDrop(cards, 'a', undefined)).toBeUndefined()
		expect(resolveDrop(cards, 'a', { column: 'archive' })).toBeUndefined()
		expect(resolveDrop(cards, 'zzz', { column: 'done' })).toBeUndefined()
	})
})
