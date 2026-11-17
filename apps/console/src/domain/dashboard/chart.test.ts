import { describe, expect, it } from 'vitest'
import { CHART_HEIGHT, rateBars, stackedBars } from '#/domain/dashboard/chart'

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
})

describe('rateBars', () => {
	it('maps 80 to nothing and 100 to the full height', () => {
		const bars = rateBars([80, 90, 100])
		expect(bars[0]?.height).toBe(0)
		expect(bars[2]?.height).toBe(CHART_HEIGHT - 4)
		expect(bars[2]?.x).toBeCloseTo(266, 5)
	})
})
