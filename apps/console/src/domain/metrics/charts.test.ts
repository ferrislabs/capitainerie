import { describe, expect, it } from 'vitest'
import {
	assignSlots,
	cumulate,
	deltaPercent,
	donutArcs,
	fillDays,
	niceMax,
	OTHER_KEY,
	previousWindow,
	topSlices,
} from '#/domain/metrics/charts'

const day = (value: string, cost: number) => ({
	day: value,
	cost_micros: cost,
	input_tokens: cost * 2,
	output_tokens: cost,
})

describe('fillDays', () => {
	it('gives one point per UTC day, zero where the API had nothing', () => {
		const points = fillDays([day('2026-11-02', 5)], {
			from: '2026-11-01T00:00:00.000Z',
			to: '2026-11-04T00:00:00.000Z',
		})
		expect(points.map((p) => [p.day, p.cost_micros])).toEqual([
			['2026-11-01', 0],
			['2026-11-02', 5],
			['2026-11-03', 0],
		])
	})

	it('includes the day of a window ending mid-day', () => {
		const points = fillDays([], {
			from: '2026-11-17T12:00:00.000Z',
			to: '2026-11-18T12:00:00.000Z',
		})
		expect(points.map((p) => p.day)).toEqual(['2026-11-17', '2026-11-18'])
	})
})

describe('niceMax', () => {
	it('rounds up to 1, 2, 5 or 10 times a power of ten', () => {
		expect(niceMax(0)).toBe(1)
		expect(niceMax(0.9)).toBe(1)
		expect(niceMax(1.2)).toBe(2)
		expect(niceMax(3.4)).toBe(5)
		expect(niceMax(7)).toBe(10)
		expect(niceMax(130)).toBe(200)
	})
})

describe('topSlices', () => {
	const inputs = ['a', 'b', 'c', 'd'].map((key, index) => ({
		key,
		label: key.toUpperCase(),
		value: (index + 1) * 10,
	}))

	it('sorts by value and folds the rest into Autres', () => {
		const slices = topSlices(inputs, 2)
		expect(slices.map((s) => s.key)).toEqual(['d', 'c', OTHER_KEY])
		expect(slices[2]?.value).toBe(30)
		expect(slices[2]?.other).toBe(true)
		expect(slices.reduce((sum, s) => sum + s.share, 0)).toBeCloseTo(1, 10)
	})

	it('drops zero values and is empty without spend', () => {
		expect(topSlices([{ key: 'a', label: 'A', value: 0 }])).toEqual([])
		expect(topSlices([])).toEqual([])
	})
})

describe('assignSlots', () => {
	it('keeps the slot of a known key when others come and go', () => {
		const slots = new Map<string, number>()
		assignSlots(slots, ['a', 'b', 'c'])
		expect([...slots.values()]).toEqual([0, 1, 2])
		assignSlots(slots, ['c', 'd'])
		expect(slots.get('c')).toBe(2)
		expect(slots.get('d')).toBe(0)
	})
})

describe('donutArcs', () => {
	it('lays the arcs end to end with a gap between them', () => {
		const arcs = donutArcs(
			[
				{ key: 'a', share: 0.5 },
				{ key: 'b', share: 0.5 },
			],
			100,
			2,
		)
		expect(arcs).toEqual([
			{ key: 'a', length: 48, offset: 0 },
			{ key: 'b', length: 48, offset: 50 },
		])
	})

	it('draws a single slice as a full ring', () => {
		expect(donutArcs([{ key: 'a', share: 1 }], 100)[0]?.length).toBe(100)
	})
})

describe('previousWindow', () => {
	it('ends where the window starts and lasts as long', () => {
		expect(
			previousWindow({
				from: '2026-11-08T00:00:00.000Z',
				to: '2026-11-18T00:00:00.000Z',
			}),
		).toEqual({
			from: '2026-10-29T00:00:00.000Z',
			to: '2026-11-08T00:00:00.000Z',
		})
	})
})

describe('cumulate', () => {
	it('adds up cost and tokens day after day', () => {
		const points = cumulate([day('2026-11-01', 5), day('2026-11-02', 3)])
		expect(points.map((p) => p.cost_micros)).toEqual([5, 8])
		expect(points.map((p) => p.input_tokens)).toEqual([10, 16])
	})
})

describe('deltaPercent', () => {
	it('compares with the reference and is null without one', () => {
		expect(deltaPercent(150, 100)).toBe(50)
		expect(deltaPercent(50, 100)).toBe(-50)
		expect(deltaPercent(10, 0)).toBeNull()
	})
})
