import { describe, expect, it } from 'vitest'
import {
	axisDates,
	CHART_HEIGHT,
	dailySeries,
	dayWindow,
	stackedBars,
} from '#/domain/dashboard/chart'

describe('stackedBars', () => {
	it('stacks the segments of a bar from the bottom', () => {
		const bars = stackedBars([[3, 1]])
		expect(bars).toHaveLength(2)
		expect(bars[0]?.series).toBe(0)
		expect((bars[0]?.y ?? 0) + (bars[0]?.height ?? 0)).toBeCloseTo(
			CHART_HEIGHT - 0.6,
			5,
		)
		expect(bars[1]?.y).toBeLessThan(bars[0]?.y ?? 0)
	})

	it('skips empty segments and scales to the tallest bar', () => {
		const bars = stackedBars([
			[2, 0],
			[4, 0],
		])
		expect(bars).toHaveLength(2)
		expect(bars[1]?.height).toBeCloseTo(CHART_HEIGHT - 4 - 0.6, 5)
	})

	it('draws nothing when every value is zero', () => {
		expect(stackedBars([[0], [0]])).toEqual([])
		expect(stackedBars([])).toEqual([])
	})
})

describe('dayWindow', () => {
	it('lists the last days ending today in UTC', () => {
		const window = dayWindow(new Date('2026-11-18T15:30:00Z'), 3)
		expect(window.days).toEqual(['2026-11-16', '2026-11-17', '2026-11-18'])
		expect(window.from).toBe('2026-11-16T00:00:00.000Z')
		expect(window.to).toBe('2026-11-19T00:00:00.000Z')
	})
})

describe('dailySeries', () => {
	it('fills the missing days with zeros', () => {
		const rows = [{ key: '2026-11-17T00:00:00Z', runs: 4, failed: 1 }]
		const series = dailySeries(rows, ['2026-11-16', '2026-11-17'], 2, (row) => [
			row.runs,
			row.failed,
		])
		expect(series).toEqual([
			[0, 0],
			[4, 1],
		])
	})
})

describe('axisDates', () => {
	it('labels the first, middle and last day', () => {
		expect(axisDates(['2026-11-04', '2026-11-05', '2026-11-06'])).toHaveLength(
			3,
		)
		expect(axisDates(['2026-11-04'])).toHaveLength(1)
	})
})
