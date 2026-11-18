import { describe, expect, it } from 'vitest'
import { AGENT_TABS, resolveAgentTab } from '#/domain/agent-team/agent-tabs'

describe('resolveAgentTab', () => {
	it('lists the nine tabs in order', () => {
		expect(AGENT_TABS.map((t) => t.id)).toEqual([
			'overview',
			'instructions',
			'skills',
			'sources',
			'team',
			'connectors',
			'runtime',
			'perms',
			'rev',
		])
	})

	it('keeps known tabs', () => {
		expect(resolveAgentTab('runtime')).toBe('runtime')
	})

	it('falls back to overview', () => {
		expect(resolveAgentTab(undefined)).toBe('overview')
		expect(resolveAgentTab('nope')).toBe('overview')
	})

	it('maps legacy segments to connectors', () => {
		expect(resolveAgentTab('tools')).toBe('connectors')
		expect(resolveAgentTab('channels')).toBe('connectors')
	})
})
