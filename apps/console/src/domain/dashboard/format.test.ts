import { describe, expect, it } from 'vitest'
import {
	formatCost,
	formatTokens,
	hueOf,
	relativeAge,
} from '#/domain/dashboard/format'

describe('formatCost', () => {
	it('converts micros to dollars', () => {
		expect(formatCost(0)).toContain('0,00')
		expect(formatCost(1_234_000)).toContain('1,23')
	})
})

describe('formatTokens', () => {
	it('abbreviates thousands and millions', () => {
		expect(formatTokens(950)).toBe('950')
		expect(formatTokens(1500)).toContain('k')
		expect(formatTokens(2_500_000)).toContain('M')
	})
})

describe('relativeAge', () => {
	const now = Date.parse('2026-11-18T12:00:00Z')
	it('formats seconds, minutes, hours and days', () => {
		expect(relativeAge('2026-11-18T11:59:50Z', now)).toBe("à l'instant")
		expect(relativeAge('2026-11-18T11:40:00Z', now)).toBe('il y a 20 min')
		expect(relativeAge('2026-11-18T09:00:00Z', now)).toBe('il y a 3 h')
		expect(relativeAge('2026-11-15T12:00:00Z', now)).toBe('il y a 3 j')
	})
})

describe('hueOf', () => {
	it('is stable and within the colour wheel', () => {
		expect(hueOf('iris')).toBe(hueOf('iris'))
		expect(hueOf('iris')).toBeLessThan(360)
	})
})
