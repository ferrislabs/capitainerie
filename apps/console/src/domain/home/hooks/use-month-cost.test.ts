import { describe, expect, it } from 'vitest'
import { monthStart } from '#/domain/home/hooks/use-month-cost'

describe('monthStart', () => {
	it('starts on the first day of the month in UTC', () => {
		expect(monthStart(new Date('2026-10-19T14:30:00Z'))).toBe(
			'2026-10-01T00:00:00.000Z',
		)
	})

	it('stays in the same month on the last instant of the month', () => {
		expect(monthStart(new Date('2026-10-31T23:59:59.999Z'))).toBe(
			'2026-10-01T00:00:00.000Z',
		)
	})

	it('moves on at the first instant of the next month', () => {
		expect(monthStart(new Date('2026-11-01T00:00:00.000Z'))).toBe(
			'2026-11-01T00:00:00.000Z',
		)
	})

	it('crosses the year boundary', () => {
		expect(monthStart(new Date('2027-01-01T00:00:00Z'))).toBe(
			'2027-01-01T00:00:00.000Z',
		)
	})
})
