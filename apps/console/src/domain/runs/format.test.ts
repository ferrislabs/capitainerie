import { describe, expect, it } from 'vitest'
import {
	formatCost,
	formatElapsed,
	formatLatency,
	formatTokens,
	runDurationMs,
} from '#/domain/runs/format'

describe('formatCost', () => {
	it('converts micros to euros with four decimals', () => {
		expect(formatCost(12_345)).toBe('0,0123 €')
		expect(formatCost(2_500_000)).toBe('2,5000 €')
		expect(formatCost(0)).toBe('0,0000 €')
	})
})

describe('formatLatency', () => {
	it('uses ms below one second and seconds above', () => {
		expect(formatLatency(850)).toBe('850 ms')
		expect(formatLatency(1234)).toBe('1,2 s')
	})
})

describe('formatTokens', () => {
	it('groups digits in fr-FR', () => {
		expect(formatTokens(1234567).replace(/\s/g, ' ')).toBe('1 234 567')
	})
})

describe('runDurationMs', () => {
	it('is the gap between start and finish', () => {
		expect(
			runDurationMs({
				started_at: '2026-10-16T09:00:00Z',
				finished_at: '2026-10-16T09:00:03Z',
			}),
		).toBe(3000)
	})

	it('is null while unfinished', () => {
		expect(runDurationMs({ started_at: '2026-10-16T09:00:00Z' })).toBeNull()
		expect(runDurationMs({})).toBeNull()
	})
})

describe('formatElapsed', () => {
	it('counts up in seconds, then minutes, then hours', () => {
		expect(formatElapsed(-500)).toBe('0 s')
		expect(formatElapsed(42_900)).toBe('42 s')
		expect(formatElapsed(65_000)).toBe('1 min 05 s')
		expect(formatElapsed(3_720_000)).toBe('1 h 02 min')
	})
})
