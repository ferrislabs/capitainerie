import { describe, expect, it } from 'vitest'
import { monthWindow } from '#/domain/home/hooks/use-month-cost'

describe('monthWindow', () => {
	it('starts on the first day of the current month', () => {
		const now = new Date(2026, 9, 19, 14, 30)
		const { from, to } = monthWindow(now)

		expect(new Date(from)).toEqual(new Date(2026, 9, 1))
		expect(to).toBe(now.toISOString())
	})
})
