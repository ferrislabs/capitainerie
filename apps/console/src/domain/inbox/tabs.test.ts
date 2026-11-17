import { describe, expect, it } from 'vitest'
import { parseInboxTab } from '#/domain/inbox/tabs'

describe('parseInboxTab', () => {
	it('keeps a known tab', () => {
		expect(parseInboxTab('non-lus')).toBe('non-lus')
	})

	it('falls back to tous', () => {
		expect(parseInboxTab(undefined)).toBe('tous')
		expect(parseInboxTab('x')).toBe('tous')
	})
})
