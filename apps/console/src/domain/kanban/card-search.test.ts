import { describe, expect, it } from 'vitest'
import { parseCardSearch } from '#/domain/kanban/card-search'

describe('parseCardSearch', () => {
	it('keeps a card identifier', () => {
		expect(parseCardSearch({ card: 'c1' })).toEqual({ card: 'c1' })
	})

	it('ignores a missing, empty or non-text value', () => {
		expect(parseCardSearch({})).toEqual({ card: undefined })
		expect(parseCardSearch({ card: '  ' })).toEqual({ card: undefined })
		expect(parseCardSearch({ card: 12 })).toEqual({ card: undefined })
	})

	it('keeps client and project names, one or several, and drops the rest', () => {
		expect(
			parseCardSearch({ client: ['Acme', 'Acme', ' ', 3], project: 'Site' }),
		).toEqual({ client: ['Acme'], project: ['Site'] })
		expect(parseCardSearch({ client: [], project: 4 })).toEqual({})
	})
})
