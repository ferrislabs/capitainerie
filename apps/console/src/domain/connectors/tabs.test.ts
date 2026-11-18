import { describe, expect, it } from 'vitest'
import { parseConnectorTab } from '#/domain/connectors/tabs'

describe('parseConnectorTab', () => {
	it('falls back to installes', () => {
		expect(parseConnectorTab(undefined)).toBe('installes')
		expect(parseConnectorTab('nope')).toBe('installes')
		expect(parseConnectorTab('archives')).toBe('archives')
	})
})
