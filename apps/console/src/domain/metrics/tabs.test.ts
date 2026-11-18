import { describe, expect, it } from 'vitest'
import { parseMetricsTab } from '#/domain/metrics/tabs'

describe('parseMetricsTab', () => {
	it('keeps a known tab', () => {
		expect(parseMetricsTab('taches')).toBe('taches')
		expect(parseMetricsTab('utilisateurs')).toBe('utilisateurs')
	})

	it('falls back to projets', () => {
		expect(parseMetricsTab(undefined)).toBe('projets')
		expect(parseMetricsTab('nope')).toBe('projets')
	})
})
