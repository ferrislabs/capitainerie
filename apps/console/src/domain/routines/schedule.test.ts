import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	defaultSchedule,
	scheduleDraftOf,
	scheduleDto,
	scheduleError,
	scheduleText,
	toggleDay,
} from '#/domain/routines/schedule'

describe('scheduleText', () => {
	it('summarises every frequency', () => {
		expect(scheduleText({ frequency: 'every_hours', hours: 48 })).toBe(
			'Toutes les 48 h',
		)
		expect(scheduleText({ frequency: 'every_hours', hours: 1 })).toBe(
			'Toutes les heures',
		)
		expect(scheduleText({ frequency: 'daily', time: '06:00' })).toBe(
			'Chaque jour 06:00',
		)
		expect(
			scheduleText({
				frequency: 'weekly',
				time: '09:30',
				days: ['mon', 'wed'],
			}),
		).toBe('Lun, Mer 09:30')
		expect(
			scheduleText({
				frequency: 'weekly',
				time: '08:00',
				days: ['mon', 'tue', 'wed', 'thu', 'fri'],
			}),
		).toBe('Jours ouvrés 08:00')
		expect(scheduleText({ frequency: 'monthly', time: '07:00' })).toBe(
			'Le 1er du mois 07:00',
		)
		expect(scheduleText({ frequency: 'cron', cron: '30 6 * * 1' })).toBe(
			'Cron 30 6 * * 1',
		)
	})
})

describe('scheduleDto', () => {
	it('sends only the fields of the chosen frequency', () => {
		const draft = { ...defaultSchedule(), days: ['mon', 'fri'] as const }
		expect(scheduleDto({ ...draft, frequency: 'weekly' })).toEqual({
			frequency: 'weekly',
			time: '08:00',
			days: ['mon', 'fri'],
		})
		expect(
			scheduleDto({ ...draft, frequency: 'every_hours', hours: 6 }),
		).toEqual({ frequency: 'every_hours', hours: 6 })
		expect(
			scheduleDto({ ...draft, frequency: 'cron', cron: ' 0 6 * * * ' }),
		).toEqual({ frequency: 'cron', cron: '0 6 * * *' })
		expect(scheduleDto({ ...draft, frequency: 'daily' })).toEqual({
			frequency: 'daily',
			time: '08:00',
		})
	})

	it('round-trips through scheduleDraftOf', () => {
		const dto: Schemas.ScheduleDto = {
			frequency: 'weekly',
			time: '10:15',
			days: ['sat'],
		}
		expect(scheduleDto(scheduleDraftOf(dto))).toEqual(dto)
		expect(scheduleDraftOf(null)).toEqual(defaultSchedule())
	})
})

describe('scheduleError', () => {
	it('validates hours, days, time and cron', () => {
		const base = defaultSchedule()
		expect(scheduleError(base)).toBeNull()
		expect(scheduleError({ ...base, days: [] })).toMatch(/au moins un jour/)
		expect(scheduleError({ ...base, time: '25:00' })).toMatch(/HH:MM/)
		expect(
			scheduleError({ ...base, frequency: 'every_hours', hours: 0 }),
		).toMatch(/entre 1 et 744/)
		expect(
			scheduleError({ ...base, frequency: 'every_hours', hours: 12 }),
		).toBeNull()
		expect(
			scheduleError({ ...base, frequency: 'cron', cron: '* * *' }),
		).toMatch(/5 champs/)
		expect(
			scheduleError({ ...base, frequency: 'cron', cron: '0 6 * * 1-5' }),
		).toBeNull()
	})
})

describe('toggleDay', () => {
	it('adds in week order and removes', () => {
		expect(toggleDay(['fri'], 'mon')).toEqual(['mon', 'fri'])
		expect(toggleDay(['mon', 'fri'], 'mon')).toEqual(['fri'])
	})
})
