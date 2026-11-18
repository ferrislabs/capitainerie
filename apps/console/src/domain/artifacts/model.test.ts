import { describe, expect, it } from 'vitest'
import {
	artifactFileName,
	kindLabel,
	prettyJson,
	safeUrl,
} from '#/domain/artifacts/model'

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

	it('names the downloaded file after the title', () => {
		expect(artifactFileName('Audit sécurité : JWKS', 'markdown')).toBe(
			'audit-securite-jwks.md',
		)
		expect(artifactFileName('Inventaire', 'json')).toBe('inventaire.json')
		expect(artifactFileName('!!!', 'markdown')).toBe('artifact.md')
	})
})
