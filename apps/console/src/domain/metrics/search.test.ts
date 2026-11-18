import { describe, expect, it } from 'vitest'
import { parseMetricsSearch } from '#/domain/metrics/search'

describe('parseMetricsSearch', () => {
	it('keeps a valid free range and the filters', () => {
		expect(
			parseMetricsSearch({
				periode: 'libre',
				du: '2026-11-01',
				au: '2026-11-18',
				agent: 'a1',
				tache: 'c1',
			}),
		).toEqual({
			periode: 'libre',
			du: '2026-11-01',
			au: '2026-11-18',
			agent: 'a1',
			tache: 'c1',
		})
	})

	it('drops a free range without both days, or reversed', () => {
		expect(parseMetricsSearch({ periode: 'libre', du: '2026-11-01' })).toEqual(
			{},
		)
		expect(
			parseMetricsSearch({
				periode: 'libre',
				du: '2026-11-18',
				au: '2026-11-01',
			}),
		).toEqual({})
		expect(
			parseMetricsSearch({ periode: 'libre', du: 'demain', au: '2026-11-01' }),
		).toEqual({})
	})

	it('ignores days outside a free range and unknown periods', () => {
		expect(parseMetricsSearch({ periode: '7j', du: '2026-11-01' })).toEqual({
			periode: '7j',
		})
		expect(parseMetricsSearch({ periode: 'nope' })).toEqual({})
	})

	it('leaves the default period out of the URL', () => {
		expect(parseMetricsSearch({ periode: '30j' })).toEqual({})
	})

	it('drops empty or non-text filters', () => {
		expect(parseMetricsSearch({ agent: '', tache: 3 })).toEqual({})
	})
})
