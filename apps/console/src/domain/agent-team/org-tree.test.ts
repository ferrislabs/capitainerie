import { beforeEach, describe, expect, it } from 'vitest'
import { buildOrgTree } from '#/domain/agent-team/org-tree'
import { demoStore, resetDemoState, setReportsTo } from '#/domain/demo'

describe('buildOrgTree', () => {
	beforeEach(resetDemoState)

	it('starts at the head of the team and indents sub-agents', () => {
		const nodes = buildOrgTree(demoStore.state)
		expect(nodes[0]).toMatchObject({ id: 'chief-of-staff', depth: 0 })
		const legal = nodes.find((n) => n.id === 'veille-legale')
		expect(legal?.depth).toBe(1)
		expect(legal?.subCount).toBe(2)
		expect(nodes.find((n) => n.id === 'registre')?.depth).toBe(2)
	})

	it('lists every agent once', () => {
		const nodes = buildOrgTree(demoStore.state)
		expect(new Set(nodes.map((n) => n.id)).size).toBe(nodes.length)
		expect(nodes).toHaveLength(demoStore.state.agents.length)
	})

	it('survives a cycle', () => {
		setReportsTo('chief-of-staff', 'Archibald')
		const nodes = buildOrgTree(demoStore.state)
		expect(new Set(nodes.map((n) => n.id)).size).toBe(nodes.length)
	})
})
