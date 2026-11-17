import { describe, expect, it } from 'vitest'
import { parseAuditTab } from '#/domain/audit/tabs'

describe('parseAuditTab', () => {
	it('keeps a known tab', () => {
		expect(parseAuditTab('couts')).toBe('couts')
		expect(parseAuditTab('chronologie')).toBe('chronologie')
	})

	it('falls back to activite', () => {
		expect(parseAuditTab(undefined)).toBe('activite')
		expect(parseAuditTab('nope')).toBe('activite')
	})
})
