import { describe, expect, it } from 'vitest'
import { filterConnectors, parseConnectorTab } from '#/domain/connectors/tabs'
import { DEMO_CONNECTORS } from '#/domain/demo'

describe('parseConnectorTab', () => {
	it('falls back to installes', () => {
		expect(parseConnectorTab(undefined)).toBe('installes')
		expect(parseConnectorTab('nope')).toBe('installes')
		expect(parseConnectorTab('outils')).toBe('outils')
	})
})

describe('filterConnectors', () => {
	it('installes keeps connectors with an account', () => {
		const list = filterConnectors('installes', DEMO_CONNECTORS)
		expect(list.length).toBeGreaterThan(0)
		expect(list.every((c) => c.accounts.length > 0)).toBe(true)
	})
	it('canaux, outils and registres filter by kind', () => {
		expect(
			filterConnectors('canaux', DEMO_CONNECTORS).every(
				(c) => c.kind === 'Canal',
			),
		).toBe(true)
		expect(
			filterConnectors('outils', DEMO_CONNECTORS).every(
				(c) => c.kind === 'Outil' || c.kind === 'MCP',
			),
		).toBe(true)
		expect(
			filterConnectors('registres', DEMO_CONNECTORS).every(
				(c) => c.kind === 'Registre',
			),
		).toBe(true)
	})
	it('tous keeps everything', () => {
		expect(filterConnectors('tous', DEMO_CONNECTORS)).toHaveLength(
			DEMO_CONNECTORS.length,
		)
	})
})
