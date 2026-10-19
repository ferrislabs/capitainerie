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
})
