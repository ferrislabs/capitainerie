import { describe, expect, it } from 'vitest'
import {
	formatRange,
	monthWeeks,
	pickDay,
	previewRange,
	shiftMonth,
} from '#/domain/metrics/calendar'

describe('monthWeeks', () => {
	it('starts on a Monday and pads with the neighbouring days', () => {
		const weeks = monthWeeks('2026-11-01')
		expect(weeks[0]?.[0]).toEqual({ day: '2026-10-26', inMonth: false })
		expect(weeks[0]?.[6]).toEqual({ day: '2026-11-01', inMonth: true })
		expect(weeks.at(-1)?.some((cell) => cell.day === '2026-11-30')).toBe(true)
		expect(weeks.every((week) => week.length === 7)).toBe(true)
	})

	it('does not add a week of the next month only', () => {
		expect(monthWeeks('2027-02-01')).toHaveLength(4)
	})
})

describe('shiftMonth', () => {
	it('moves by whole months across a year', () => {
		expect(shiftMonth('2026-01-15', -1)).toBe('2025-12-01')
		expect(shiftMonth('2026-11-01', 2)).toBe('2027-01-01')
	})
})

describe('pickDay', () => {
	it('sets the start, then the end, then starts over', () => {
		const first = pickDay({ from: '', to: '' }, '2026-11-05')
		expect(first).toEqual({ from: '2026-11-05', to: '' })
		const second = pickDay(first, '2026-11-09')
		expect(second).toEqual({ from: '2026-11-05', to: '2026-11-09' })
		expect(pickDay(second, '2026-11-20')).toEqual({
			from: '2026-11-20',
			to: '',
		})
	})

	it('puts an earlier second click before the start', () => {
		expect(pickDay({ from: '2026-11-09', to: '' }, '2026-11-03')).toEqual({
			from: '2026-11-03',
			to: '2026-11-09',
		})
	})
})

describe('previewRange', () => {
	it('previews the end under the pointer while only the start is set', () => {
		expect(previewRange({ from: '2026-11-05', to: '' }, '2026-11-08')).toEqual({
			from: '2026-11-05',
			to: '2026-11-08',
		})
		expect(
			previewRange({ from: '2026-11-05', to: '2026-11-06' }, '2026-11-08'),
		).toEqual({
			from: '2026-11-05',
			to: '2026-11-06',
		})
	})
})

describe('formatRange', () => {
	it('prints both bounds in French', () => {
		const label = formatRange({ from: '2026-11-01', to: '2026-11-18' })
		expect(label).toContain('2026')
		expect(label).toContain('–')
		expect(formatRange({ from: '', to: '' })).toBe('Choisir des dates')
	})
})
