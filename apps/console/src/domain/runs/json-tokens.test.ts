import { describe, expect, it } from 'vitest'
import { prettyJson, tokenizeJson } from '#/domain/runs/json-tokens'

describe('prettyJson', () => {
	it('indents with two spaces', () => {
		expect(prettyJson({ a: [1, 2] })).toBe('{\n  "a": [\n    1,\n    2\n  ]\n}')
	})
})

describe('tokenizeJson', () => {
	it('tells keys from string values', () => {
		const tokens = tokenizeJson('{\n  "branch": "main"\n}')

		expect(tokens.filter((token) => token.kind !== 'space')).toEqual([
			{ kind: 'punct', text: '{' },
			{ kind: 'key', text: '"branch"' },
			{ kind: 'punct', text: ':' },
			{ kind: 'string', text: '"main"' },
			{ kind: 'punct', text: '}' },
		])
	})

	it('recognises numbers and literals', () => {
		const kinds = tokenizeJson('[1.5, -2, true, false, null]')
			.filter((token) => token.kind !== 'space' && token.kind !== 'punct')
			.map((token) => [token.kind, token.text])

		expect(kinds).toEqual([
			['number', '1.5'],
			['number', '-2'],
			['literal', 'true'],
			['literal', 'false'],
			['literal', 'null'],
		])
	})

	it('keeps escaped quotes inside one string token', () => {
		const tokens = tokenizeJson('"dit \\"bonjour\\""')

		expect(tokens).toEqual([{ kind: 'string', text: '"dit \\"bonjour\\""' }])
	})

	it('gives back the exact text when the tokens are joined', () => {
		const text = prettyJson({ a: { b: [true, null, 'x: y', 3] }, c: 'd' })

		expect(
			tokenizeJson(text)
				.map((token) => token.text)
				.join(''),
		).toBe(text)
	})
})
