import { describe, expect, it } from 'vitest'
import { type CustomRange, periodWindow } from '#/domain/metrics/model'

const NOW = new Date('2026-11-18T12:00:00Z')
const NO_RANGE: CustomRange = { from: '', to: '' }

describe('periodWindow', () => {
	it('ends now and starts N days earlier for the rolling periods', () => {
		expect(periodWindow('7j', NOW, NO_RANGE)).toEqual({
			from: '2026-11-11T12:00:00.000Z',
			to: '2026-11-18T12:00:00.000Z',
		})
		expect(periodWindow('30j', NOW, NO_RANGE)).toEqual({
			from: '2026-10-19T12:00:00.000Z',
			to: '2026-11-18T12:00:00.000Z',
		})
	})

	it('covers the current calendar month', () => {
		expect(periodWindow('mois', NOW, NO_RANGE)).toEqual({
			from: '2026-11-01T00:00:00.000Z',
			to: '2026-12-01T00:00:00.000Z',
		})
	})

	it('includes the last day of a custom range', () => {
		const window = periodWindow('libre', NOW, {
			from: '2026-11-01',
			to: '2026-11-03',
		})
		expect(window).toEqual({
			from: '2026-11-01T00:00:00.000Z',
			to: '2026-11-04T00:00:00.000Z',
		})
	})

	it('falls back to 30 days while a custom bound is missing', () => {
		expect(periodWindow('libre', NOW, { from: '2026-11-01', to: '' })).toEqual(
			periodWindow('30j', NOW, NO_RANGE),
		)
	})
})
