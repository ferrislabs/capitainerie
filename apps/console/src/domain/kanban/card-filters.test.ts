import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	type CardFilters,
	filterCards,
	filterOptions,
	hasFilters,
	NO_FILTERS,
} from '#/domain/kanban/card-filters'

function card(
	id: string,
	extra: {
		client?: string
		project?: string
		priority?: Schemas.PriorityDto
		assignee?: string
		labels?: string[]
	} = {},
): Schemas.CardResponse {
	return {
		id,
		title: id,
		description: '',
		column: 'todo',
		provenance: null,
		created_by_run: null,
		created_by_subject: 'ada',
		created_by_agent: null,
		version: 1,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
		client: extra.client
			? { id: `c-${extra.client}`, name: extra.client }
			: null,
		project: extra.project
			? { id: `p-${extra.project}`, name: extra.project }
			: null,
		acceptance_criteria: [],
		priority: extra.priority ?? 'normal',
		due_date: null,
		assignee_subject: extra.assignee ?? null,
		labels: extra.labels ?? [],
		estimate: null,
	}
}

const CARDS = [
	card('a', { client: 'Acme', project: 'Site', priority: 'urgent' }),
	card('b', {
		client: 'Acme',
		project: 'App',
		assignee: 'u1',
		labels: ['Infra'],
	}),
	card('c', {
		client: 'Globex',
		project: 'Site',
		priority: 'high',
		assignee: 'u2',
	}),
	card('d', { labels: ['infra', 'doc'] }),
]

const NAMES = new Map([
	['u1', 'Claire Martin'],
	['u2', 'Bastien'],
])

const ids = (filters: Partial<CardFilters>) =>
	filterCards(CARDS, { ...NO_FILTERS, ...filters }, NAMES).map(
		(item) => item.id,
	)

describe('card filters', () => {
	it('keeps everything without a filter', () => {
		expect(hasFilters(NO_FILTERS)).toBe(false)
		expect(filterCards(CARDS, NO_FILTERS)).toBe(CARDS)
	})

	it('combines values of a chip with or, and chips with and', () => {
		expect(ids({ clients: ['Acme'] })).toEqual(['a', 'b'])
		expect(ids({ clients: ['Acme', 'Globex'] })).toEqual(['a', 'b', 'c'])
		expect(ids({ clients: ['Acme'], projects: ['Site'] })).toEqual(['a'])
		expect(ids({ projects: ['Site'] })).toEqual(['a', 'c'])
		expect(ids({ clients: ['Inconnu'] })).toEqual([])
	})

	it('filters by priority, by assignee name and by label without case', () => {
		expect(ids({ priorities: ['Urgente'] })).toEqual(['a'])
		expect(ids({ priorities: ['Urgente', 'Haute'] })).toEqual(['a', 'c'])
		expect(ids({ priorities: ['Normale'] })).toEqual(['b', 'd'])
		expect(ids({ assignees: ['Claire Martin'] })).toEqual(['b'])
		expect(ids({ assignees: ['Claire Martin', 'Bastien'] })).toEqual(['b', 'c'])
		expect(ids({ labels: ['INFRA'] })).toEqual(['b', 'd'])
		expect(ids({ labels: ['doc'], assignees: ['Claire Martin'] })).toEqual([])
	})

	it('falls back to the subject for an assignee who is not a known member', () => {
		const stranger = [card('x', { assignee: 'u9' })]
		expect(
			filterCards(stranger, { ...NO_FILTERS, assignees: ['u9'] }, NAMES),
		).toHaveLength(1)
	})

	it('offers the values present, sorted, plus the ones already chosen', () => {
		expect(filterOptions(CARDS, NO_FILTERS, NAMES)).toEqual({
			clients: ['Acme', 'Globex'],
			projects: ['App', 'Site'],
			priorities: ['Urgente', 'Haute', 'Normale'],
			assignees: ['Bastien', 'Claire Martin'],
			labels: ['doc', 'Infra'],
		})
		const kept = filterOptions([], {
			...NO_FILTERS,
			clients: ['Acme'],
			priorities: ['Basse'],
		})
		expect(kept.clients).toEqual(['Acme'])
		expect(kept.priorities).toEqual(['Basse'])
	})
})
