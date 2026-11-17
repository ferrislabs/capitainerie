import { describe, expect, it } from 'vitest'
import {
	DEFAULT_SETTINGS,
	maskedSecret,
	nameError,
	scheduleError,
	slugify,
	toggleDay,
	uniqueId,
	webhookUrl,
} from '#/domain/routines/routine-draft'
import { defaultSchedule } from '#/domain/routines/schedule'

describe('slugify', () => {
	it('strips accents and punctuation', () => {
		expect(slugify('Marketing : déclencheurs de publication')).toBe(
			'marketing-declencheurs-de-publication',
		)
		expect(slugify('Relance des décisions ouvertes (J+2)')).toBe(
			'relance-des-decisions-ouvertes-j-2',
		)
	})
})

describe('uniqueId', () => {
	it('suffixes an id that is already taken', () => {
		expect(uniqueId('a', ['a', 'a-2'])).toBe('a-3')
		expect(uniqueId('b', ['a'])).toBe('b')
		expect(uniqueId('', [])).toBe('routine')
	})
})

describe('validation', () => {
	it('requires a name', () => {
		expect(nameError('  ')).not.toBeNull()
		expect(nameError('Ok')).toBeNull()
	})
	it('requires a weekday for weekly schedules only', () => {
		const weekly = { ...defaultSchedule(), days: [] }
		expect(scheduleError(weekly)).toBe('Choisis au moins un jour')
		expect(scheduleError({ ...weekly, frequency: 'Tous les jours' })).toBeNull()
	})
})

describe('toggleDay', () => {
	it('adds and removes days keeping week order', () => {
		expect(toggleDay(['mer'], 'lun')).toEqual(['lun', 'mer'])
		expect(toggleDay(['lun', 'mer'], 'lun')).toEqual(['mer'])
	})
})

describe('webhook helpers', () => {
	it('builds the url and masks the secret', () => {
		expect(webhookUrl('Analyse board', 'abc123')).toBe(
			'https://paperclip.getnatalia.com/hooks/routines/analyse-board-abc123',
		)
		expect(maskedSecret('wxyz')).toBe('whsec_••••••••wxyz')
	})
})

describe('DEFAULT_SETTINGS', () => {
	it('starts with an empty name and the mockup defaults', () => {
		expect(DEFAULT_SETTINGS.name).toBe('')
		expect(DEFAULT_SETTINGS.titleTemplate).toBe('{nom} · {date}')
	})
})
