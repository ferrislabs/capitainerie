import { describe, expect, it } from 'vitest'
import { parseAgentFilter } from '#/domain/agent-team/agent-filters'

describe('parseAgentFilter', () => {
	it('keeps known filters and falls back to all', () => {
		expect(parseAgentFilter('actifs')).toBe('actifs')
		expect(parseAgentFilter('pause')).toBe('pause')
		expect(parseAgentFilter('archives')).toBe('archives')
		expect(parseAgentFilter('nope')).toBe('tous')
		expect(parseAgentFilter(undefined)).toBe('tous')
	})
})
