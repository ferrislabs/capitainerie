import { describe, expect, it } from 'vitest'
import { stackedBars } from '#/domain/audit/stacked-bars'

const options = { width: 100, height: 54, barWidth: 10 }

describe('stackedBars', () => {
	it('returns nothing without data', () => {
		expect(stackedBars([], options)).toEqual([])
		expect(stackedBars([[0, 0]], options)).toEqual([])
	})

	it('stacks segments upwards and skips zero values', () => {
		const segments = stackedBars(
			[
				[10, 0],
				[5, 5],
			],
			options,
		)
		expect(segments.map((s) => s.series)).toEqual([0, 0, 1])
		const first = segments[0]
		expect(first?.y).toBeCloseTo(4)
		expect(first?.height).toBeCloseTo(49.4)
	})

	it('spreads columns across the width', () => {
		const segments = stackedBars([[1], [1], [1]], options)
		expect(segments.map((s) => s.x)).toEqual([0, 45, 90])
	})
})
