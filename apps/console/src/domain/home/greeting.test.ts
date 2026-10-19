import { describe, expect, it } from 'vitest'
import { firstName, greeting, greetingTitle } from '#/domain/home/greeting'

function at(hour: number, minute = 0) {
	return new Date(2026, 9, 19, hour, minute)
}

describe('greeting', () => {
	it('says Bonjour before 18:00', () => {
		expect(greeting(at(0))).toBe('Bonjour')
		expect(greeting(at(9))).toBe('Bonjour')
		expect(greeting(at(17, 59))).toBe('Bonjour')
	})

	it('says Bonsoir from 18:00', () => {
		expect(greeting(at(18))).toBe('Bonsoir')
		expect(greeting(at(23, 59))).toBe('Bonsoir')
	})
})

describe('firstName', () => {
	it('takes the first word of the display name', () => {
		expect(firstName({ display_name: 'Ada Lovelace', username: 'ada' })).toBe(
			'Ada',
		)
	})

	it('falls back to the username', () => {
		expect(firstName({ display_name: ' ', username: 'ada' })).toBe('ada')
		expect(firstName({ display_name: null, username: 'ada' })).toBe('ada')
	})

	it('is undefined without a name', () => {
		expect(firstName({})).toBeUndefined()
		expect(firstName(undefined)).toBeUndefined()
	})
})

describe('greetingTitle', () => {
	it('joins the greeting and the first name', () => {
		expect(greetingTitle(at(10), 'Ada')).toBe('Bonjour, Ada')
		expect(greetingTitle(at(20), 'Ada')).toBe('Bonsoir, Ada')
	})

	it('drops the name when unknown', () => {
		expect(greetingTitle(at(10), undefined)).toBe('Bonjour')
	})
})
