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
})
