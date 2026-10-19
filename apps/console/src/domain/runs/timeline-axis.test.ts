import { describe, expect, it } from 'vitest'
import {
	blockPosition,
	buildTicks,
	computeAxis,
	formatTick,
	MIN_TOTAL_MS,
	tickStepMs,
} from '#/domain/runs/timeline-axis'

const T0 = Date.parse('2026-10-19T10:00:00Z')

function at(seconds: number): string {
	return new Date(T0 + seconds * 1000).toISOString()
}

describe('formatTick', () => {
	it('writes seconds, minutes and hours the way a person reads them', () => {
		expect(formatTick(0)).toBe('0 s')
		expect(formatTick(5_000)).toBe('5 s')
		expect(formatTick(60_000)).toBe('1 min')
		expect(formatTick(90_000)).toBe('1 min 30 s')
		expect(formatTick(600_000)).toBe('10 min')
		expect(formatTick(3_600_000)).toBe('1 h')
		expect(formatTick(5_400_000)).toBe('1 h 30 min')
	})
})

describe('tickStepMs', () => {
	it('keeps the number of graduations readable', () => {
		expect(tickStepMs(4_000)).toBe(1_000)
		expect(tickStepMs(40_000)).toBe(5_000)
		expect(tickStepMs(100_000)).toBe(15_000)
		expect(tickStepMs(10 * 60_000)).toBe(2 * 60_000)
		expect(tickStepMs(5 * 3_600_000)).toBe(3_600_000)
	})

	it('never goes below one second, whatever the span', () => {
		expect(tickStepMs(0)).toBe(1_000)
	})
})

describe('buildTicks', () => {
	it('starts at zero and places each tick as a share of the axis', () => {
		const ticks = buildTicks(20_000)

		expect(ticks[0]).toEqual({ ms: 0, leftPct: 0, label: '0 s' })
		expect(ticks.map((tick) => tick.label)).toEqual([
			'0 s',
			'5 s',
			'10 s',
			'15 s',
			'20 s',
		])
		expect(ticks[2].leftPct).toBe(50)
	})
})

describe('computeAxis', () => {
	const run = {
		created_at: at(-2),
		started_at: at(0),
		finished_at: at(30),
	}

	it('spans the run from its start to its end when it is over', () => {
		const axis = computeAxis(run, [], T0 + 999_000, false)

		expect(axis).toEqual({ startMs: T0, totalMs: 30_000 })
	})

	it('stretches to now while the run is active', () => {
		const axis = computeAxis(
			{ created_at: at(-2), started_at: at(0), finished_at: null },
			[],
			T0 + 12_000,
			true,
		)

		expect(axis).toEqual({ startMs: T0, totalMs: 12_000 })
	})

	it('starts at the creation time while the run still waits in the queue', () => {
		const axis = computeAxis(
			{ created_at: at(-5), started_at: null, finished_at: null },
			[],
			T0,
			true,
		)

		expect(axis).toEqual({ startMs: T0 - 5_000, totalMs: 5_000 })
	})

	it('widens to a step that falls outside the run bounds', () => {
		const axis = computeAxis(
			run,
			[{ started_at: at(-1), finished_at: at(40), status: 'succeeded' }],
			T0 + 999_000,
			false,
		)

		expect(axis).toEqual({ startMs: T0 - 1_000, totalMs: 41_000 })
	})

	it('never collapses to a zero-length axis', () => {
		const axis = computeAxis(
			{ created_at: at(0), started_at: at(0), finished_at: at(0) },
			[],
			T0,
			false,
		)

		expect(axis.totalMs).toBe(MIN_TOTAL_MS)
	})
})

describe('blockPosition', () => {
	const axis = { startMs: T0, totalMs: 40_000 }

	it('places a finished step by its start and end', () => {
		const position = blockPosition(
			{ started_at: at(10), finished_at: at(30), status: 'succeeded' },
			axis,
			T0 + 40_000,
		)

		expect(position).toEqual({ leftPct: 25, widthPct: 50 })
	})

	it('stretches a running step to now', () => {
		const position = blockPosition(
			{ started_at: at(20), finished_at: null, status: 'running' },
			axis,
			T0 + 30_000,
		)

		expect(position).toEqual({ leftPct: 50, widthPct: 25 })
	})

	it('gives a step without an end no width, so the view can draw a mark', () => {
		const position = blockPosition(
			{ started_at: at(8), finished_at: null, status: 'skipped' },
			axis,
			T0 + 40_000,
		)

		expect(position).toEqual({ leftPct: 20, widthPct: 0 })
	})

	it('keeps a block inside the axis', () => {
		const position = blockPosition(
			{ started_at: at(30), finished_at: at(90), status: 'succeeded' },
			axis,
			T0 + 100_000,
		)

		expect(position).toEqual({ leftPct: 75, widthPct: 25 })
	})
})
