import { describe, expect, it } from 'vitest'
import {
	DAYS,
	FREQUENCIES,
	formatRun,
	nextRuns,
	parseSchedule,
	type Schedule,
	scheduleText,
} from '#/domain/routines/schedule'

const NOW = new Date(2026, 10, 17, 10, 0, 0)

const base: Schedule = {
	frequency: 'Toutes les semaines',
	hours: 24,
	time: '08:00',
	days: ['lun'],
	cron: '',
}

const stamps = (dates: ReadonlyArray<Date>) =>
	dates.map((d) => [
		d.getMonth() + 1,
		d.getDate(),
		d.getHours(),
		d.getMinutes(),
	])

describe('FREQUENCIES', () => {
	it('lists the five frequencies in order', () => {
		expect(FREQUENCIES).toEqual([
			'Toutes les N heures',
			'Tous les jours',
			'Toutes les semaines',
			'Tous les mois',
			'Expression cron',
		])
		expect(DAYS).toHaveLength(7)
	})
})

describe('parseSchedule', () => {
	it('reads a single weekday', () => {
		expect(parseSchedule('Lun. 06:30')).toMatchObject({
			frequency: 'Toutes les semaines',
			days: ['lun'],
			time: '06:30',
		})
		expect(parseSchedule('Mar. 09:30').days).toEqual(['mar'])
	})
	it('reads daily', () => {
		expect(parseSchedule('Chaque jour 06:00')).toMatchObject({
			frequency: 'Tous les jours',
			time: '06:00',
		})
	})
	it('reads working days', () => {
		expect(parseSchedule('Jours ouvrés 08:00')).toMatchObject({
			frequency: 'Toutes les semaines',
			days: ['lun', 'mar', 'mer', 'jeu', 'ven'],
			time: '08:00',
		})
	})
	it('reads an hour interval', () => {
		expect(parseSchedule('Toutes les 48 h')).toMatchObject({
			frequency: 'Toutes les N heures',
			hours: 48,
		})
	})
	it('reads monthly', () => {
		expect(parseSchedule('Le 1er du mois').frequency).toBe('Tous les mois')
	})
})

describe('scheduleText', () => {
	it('round-trips the fixture texts', () => {
		for (const text of [
			'Lun 06:30',
			'Chaque jour 06:00',
			'Jours ouvrés 08:00',
			'Toutes les 48 h',
		]) {
			expect(scheduleText(parseSchedule(text))).toBe(text)
		}
	})
	it('describes monthly and cron schedules', () => {
		expect(scheduleText({ ...base, frequency: 'Tous les mois' })).toBe(
			'Le 1er du mois 08:00',
		)
		expect(scheduleText({ ...base, frequency: 'Expression cron' })).toBe(
			'* * * * *',
		)
		expect(
			scheduleText({
				...base,
				frequency: 'Expression cron',
				cron: '30 6 * * 1',
			}),
		).toBe('30 6 * * 1')
	})
	it('says when no weekday is chosen', () => {
		expect(scheduleText({ ...base, days: [] })).toBe('Aucun jour 08:00')
	})
})

describe('nextRuns', () => {
	it('computes weekly runs on the chosen days', () => {
		expect(stamps(nextRuns({ ...base, time: '06:30' }, NOW, 3))).toEqual([
			[11, 23, 6, 30],
			[11, 30, 6, 30],
			[12, 7, 6, 30],
		])
	})
	it('computes daily runs after now', () => {
		expect(
			stamps(
				nextRuns(
					{ ...base, frequency: 'Tous les jours', time: '06:00' },
					NOW,
					3,
				),
			),
		).toEqual([
			[11, 18, 6, 0],
			[11, 19, 6, 0],
			[11, 20, 6, 0],
		])
	})
	it('keeps today when the time is still ahead', () => {
		expect(
			stamps(
				nextRuns(
					{ ...base, frequency: 'Tous les jours', time: '18:00' },
					NOW,
					1,
				),
			),
		).toEqual([[11, 17, 18, 0]])
	})
	it('skips weekends for working days', () => {
		const t = parseSchedule('Jours ouvrés 08:00')
		expect(stamps(nextRuns(t, new Date(2026, 10, 20, 10, 0), 3))).toEqual([
			[11, 23, 8, 0],
			[11, 24, 8, 0],
			[11, 25, 8, 0],
		])
	})
	it('steps by the hour interval from the next full hour', () => {
		expect(
			stamps(
				nextRuns(
					{ ...base, frequency: 'Toutes les N heures', hours: 24 },
					NOW,
					3,
				),
			),
		).toEqual([
			[11, 17, 11, 0],
			[11, 18, 11, 0],
			[11, 19, 11, 0],
		])
	})
	it('computes monthly runs on the first', () => {
		expect(
			stamps(nextRuns({ ...base, frequency: 'Tous les mois' }, NOW, 3)),
		).toEqual([
			[12, 1, 8, 0],
			[1, 1, 8, 0],
			[2, 1, 8, 0],
		])
	})
	it('returns nothing for cron or without days', () => {
		expect(nextRuns({ ...base, frequency: 'Expression cron' }, NOW, 3)).toEqual(
			[],
		)
		expect(nextRuns({ ...base, days: [] }, NOW, 3)).toEqual([])
	})
})

describe('formatRun', () => {
	it('formats in French with weekday and time', () => {
		const text = formatRun(new Date(2026, 10, 18, 6, 30))
		expect(text).toContain('18')
		expect(text).toContain('06:30')
	})
})
