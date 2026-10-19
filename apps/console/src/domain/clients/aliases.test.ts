import { describe, expect, it } from 'vitest'
import {
	formatAliases,
	parseAliases,
	validateAliases,
	validateName,
	validateText,
} from '#/domain/clients/aliases'

describe('aliases', () => {
	it('splits on commas and line breaks, trims and drops case-insensitive duplicates', () => {
		expect(parseAliases(' ACME SA, acme sa\nAcme Corp,, ')).toEqual([
			'ACME SA',
			'Acme Corp',
		])
		expect(parseAliases('')).toEqual([])
	})

	it('formats them back on one line', () => {
		expect(formatAliases(['A', 'B'])).toBe('A, B')
	})

	it('bounds the count and the length', () => {
		expect(validateAliases(['a'])).toBeUndefined()
		expect(
			validateAliases(Array.from({ length: 11 }, (_, n) => `A${n}`)),
		).toBeDefined()
		expect(validateAliases(['x'.repeat(121)])).toBeDefined()
	})

	it('requires a name of 120 characters at most', () => {
		expect(validateName('  ')).toBeDefined()
		expect(validateName('Acme')).toBeUndefined()
		expect(validateName('é'.repeat(121))).toBeDefined()
	})

	it('bounds free text at 5000 characters', () => {
		expect(validateText('x'.repeat(5000))).toBeUndefined()
		expect(validateText('x'.repeat(5001))).toBeDefined()
	})
})
