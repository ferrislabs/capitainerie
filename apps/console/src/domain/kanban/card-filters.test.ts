import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	filterCards,
	filterOptions,
	hasFilters,
	NO_FILTERS,
} from '#/domain/kanban/card-filters'

function card(
	id: string,
	client?: string,
	project?: string,
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
		client: client ? { id: `c-${client}`, name: client } : null,
		project: project ? { id: `p-${project}`, name: project } : null,
	}
}

const CARDS = [
	card('a', 'Acme', 'Site'),
	card('b', 'Acme', 'App'),
	card('c', 'Globex', 'Site'),
	card('d'),
]

describe('card filters', () => {
	it('keeps everything without a filter', () => {
		expect(hasFilters(NO_FILTERS)).toBe(false)
		expect(filterCards(CARDS, NO_FILTERS)).toBe(CARDS)
	})

	it('combines values of a chip with or, and chips with and', () => {
		const ids = (clients: string[], projects: string[]) =>
			filterCards(CARDS, { clients, projects }).map((item) => item.id)
		expect(ids(['Acme'], [])).toEqual(['a', 'b'])
		expect(ids(['Acme', 'Globex'], [])).toEqual(['a', 'b', 'c'])
		expect(ids(['Acme'], ['Site'])).toEqual(['a'])
		expect(ids([], ['Site'])).toEqual(['a', 'c'])
		expect(ids(['Inconnu'], [])).toEqual([])
	})

	it('offers the names present, sorted, plus the ones already chosen', () => {
		expect(filterOptions(CARDS, NO_FILTERS)).toEqual({
			clients: ['Acme', 'Globex'],
			projects: ['App', 'Site'],
		})
		expect(
			filterOptions([], { clients: ['Acme'], projects: [] }).clients,
		).toEqual(['Acme'])
	})
})
