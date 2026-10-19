import { describe, expect, it } from 'vitest'
import { DEFAULT_RETURN_TO, safeReturnTo } from '#/lib/return-to'

describe('safeReturnTo', () => {
	it('lands on the home page by default', () => {
		expect(DEFAULT_RETURN_TO).toBe('/')
	})

	it('keeps a local path with its query', () => {
		expect(safeReturnTo({ returnTo: '/kanban?card=1' })).toBe('/kanban?card=1')
	})

	it('falls back when the state is missing or malformed', () => {
		expect(safeReturnTo(undefined)).toBe(DEFAULT_RETURN_TO)
		expect(safeReturnTo({ returnTo: 42 })).toBe(DEFAULT_RETURN_TO)
	})

	it('refuses to leave the console', () => {
		expect(safeReturnTo({ returnTo: 'https://evil.test' })).toBe(
			DEFAULT_RETURN_TO,
		)
		expect(safeReturnTo({ returnTo: '//evil.test/path' })).toBe(
			DEFAULT_RETURN_TO,
		)
	})

	it('never returns to the callback itself', () => {
		expect(safeReturnTo({ returnTo: '/callback?code=x' })).toBe(
			DEFAULT_RETURN_TO,
		)
	})
})
