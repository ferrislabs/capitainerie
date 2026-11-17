import { describe, expect, it } from 'vitest'
import { globToRegExp, testValue } from '#/domain/sources/glob'

const source = {
	patterns: ['iam.*', 'auth.*'],
	exclusions: ['*.getnatalia.com'],
}

describe('globToRegExp', () => {
	it('turns * into .+ and ignores case', () => {
		expect(globToRegExp('iam.*').test('IAM.acme.fr')).toBe(true)
	})

	it('escapes regex characters', () => {
		expect(globToRegExp('a.b').test('axb')).toBe(false)
		expect(globToRegExp('a+b').test('a+b')).toBe(true)
	})

	it('anchors the pattern', () => {
		expect(globToRegExp('iam.*').test('x.iam.acme.fr')).toBe(false)
	})
})

describe('testValue', () => {
	it('returns null for an empty value', () => {
		expect(testValue(source, '   ')).toBeNull()
	})

	it('matches a pattern', () => {
		expect(testValue(source, 'iam.acme.fr')).toEqual({
			kind: 'match',
			pattern: 'iam.*',
		})
	})

	it('lets exclusions win over patterns', () => {
		expect(testValue(source, 'auth.getnatalia.com')).toEqual({
			kind: 'excluded',
			by: '*.getnatalia.com',
		})
	})

	it('returns none when nothing matches', () => {
		expect(testValue(source, 'login.acme.fr')).toEqual({ kind: 'none' })
	})
})
