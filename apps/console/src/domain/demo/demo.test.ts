import { describe, expect, it } from 'vitest'
import { DEMO_AGENTS } from '#/domain/demo'

describe('demo fixtures', () => {
	it('gives every agent a unique id', () => {
		const ids = DEMO_AGENTS.map((agent) => agent.id)
		expect(new Set(ids).size).toBe(ids.length)
	})

	it('keeps the reporting line inside the team', () => {
		const names = new Set(
			DEMO_AGENTS.map((a) => a.name.replace(/\s*\(.*\)/, '')),
		)
		for (const agent of DEMO_AGENTS)
			if (agent.reportsTo) expect(names.has(agent.reportsTo)).toBe(true)
	})
})
