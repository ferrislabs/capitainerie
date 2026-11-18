import { describe, expect, it } from 'vitest'
import { resolveTab } from '#/domain/sources/tabs'

describe('resolveTab', () => {
	it('keeps known tabs', () => {
		expect(resolveTab('evenements')).toBe('evenements')
	})

	it('falls back to apercu', () => {
		expect(resolveTab(undefined)).toBe('apercu')
		expect(resolveTab('pipeline')).toBe('apercu')
	})
})
