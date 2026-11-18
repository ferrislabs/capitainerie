import { describe, expect, it } from 'vitest'
import { kindLabel, prettyJson, safeUrl } from '#/domain/artifacts/model'

describe('artifacts model', () => {
	it('indents JSON and keeps invalid content as is', () => {
		expect(prettyJson('{"a":[1,2]}')).toBe('{\n  "a": [\n    1,\n    2\n  ]\n}')
		expect(prettyJson('not json')).toBe('not json')
	})

	it('only opens http and https URLs', () => {
		expect(safeUrl('https://example.com/a')).toBe('https://example.com/a')
		expect(safeUrl('javascript:alert(1)')).toBeNull()
		expect(safeUrl('not a url')).toBeNull()
		expect(safeUrl(null)).toBeNull()
	})

	it('labels the kinds in French', () => {
		expect(kindLabel('markdown')).toBe('Document')
		expect(kindLabel('link')).toBe('Lien')
	})
})
