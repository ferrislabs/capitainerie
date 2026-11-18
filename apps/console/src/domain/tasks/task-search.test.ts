import { describe, expect, it } from 'vitest'
import { parseTasksSearch } from '#/domain/tasks/task-search'

describe('parseTasksSearch', () => {
	it('defaults to the list view and an empty query', () => {
		expect(parseTasksSearch({})).toEqual({ vue: 'liste', q: '' })
	})

	it('keeps the board view and the query', () => {
		expect(parseTasksSearch({ vue: 'tableau', q: 'nat' })).toEqual({
			vue: 'tableau',
			q: 'nat',
		})
	})

	it('falls back on invalid values', () => {
		expect(parseTasksSearch({ vue: 'x', q: 3 })).toEqual({
			vue: 'liste',
			q: '',
		})
	})

	it('keeps valid filter, sort and grouping values', () => {
		expect(
			parseTasksSearch({
				statut: 'done',
				agent: 'a1',
				projet: 'p1',
				tri: 'priorite',
				groupe: 'projet',
			}),
		).toEqual({
			vue: 'liste',
			q: '',
			statut: 'done',
			agent: 'a1',
			projet: 'p1',
			tri: 'priorite',
			groupe: 'projet',
		})
	})

	it('drops unknown filter, sort and grouping values', () => {
		expect(
			parseTasksSearch({ statut: 'blocked', agent: '', tri: 'x', groupe: 1 }),
		).toEqual({ vue: 'liste', q: '' })
	})
})
