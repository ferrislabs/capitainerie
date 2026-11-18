import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	activeAgentCount,
	attentionItems,
	cardStats,
	recentCards,
} from '#/domain/dashboard/stats'

const card = (
	id: string,
	column: Schemas.ColumnDto,
	updated_at = '2026-11-18T10:00:00Z',
) => ({ id, column, updated_at, title: `Carte ${id}` }) as Schemas.CardResponse

const run = (status: Schemas.RunStatusResponse, failure?: string) =>
	({ id: status, status, failure }) as Schemas.RunResponse

describe('cardStats', () => {
	it('counts open, in progress and proposed cards', () => {
		const stats = cardStats([
			card('1', 'todo'),
			card('2', 'in_progress'),
			card('3', 'proposed'),
			card('4', 'done'),
			card('5', 'rejected'),
		])
		expect(stats).toEqual({ open: 2, inProgress: 1, proposed: 1 })
	})

	it('is all zero without cards', () => {
		expect(cardStats([])).toEqual({ open: 0, inProgress: 0, proposed: 0 })
	})
})

describe('activeAgentCount', () => {
	it('counts distinct agents of the running steps', () => {
		const item = (agent?: string) =>
			({ current_step: agent ? { agent } : null }) as Schemas.ActiveRunResponse
		expect(activeAgentCount([item('a'), item('a'), item('b'), item()])).toBe(2)
	})
})

describe('recentCards', () => {
	it('sorts by last update, hides rejected cards and caps the list', () => {
		const cards = [
			card('old', 'todo', '2026-11-01T00:00:00Z'),
			card('new', 'todo', '2026-11-18T00:00:00Z'),
			card('rej', 'rejected', '2026-11-19T00:00:00Z'),
		]
		expect(recentCards(cards, 2).map((c) => c.id)).toEqual(['new', 'old'])
	})
})

describe('attentionItems', () => {
	it('reports failed runs and proposed cards', () => {
		const items = attentionItems(
			[card('1', 'proposed')],
			[run('failed', 'boom'), run('succeeded')],
		)
		expect(items.map((i) => i.severity)).toEqual(['crit', 'warn'])
		expect(items[0]?.detail).toBe('boom')
		expect(items[1]?.target).toBe('tasks')
	})

	it('is empty when nothing needs attention', () => {
		expect(attentionItems([card('1', 'todo')], [run('succeeded')])).toEqual([])
	})
})
