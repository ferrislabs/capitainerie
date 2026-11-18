import { describe, expect, it } from 'vitest'
import {
	ancestorsOf,
	buildOrgTree,
	subAgents,
} from '#/domain/agent-team/org-tree'
import {
	CHIEF,
	DEV,
	LEGAL,
	REGISTRE,
	TEAM,
	teamAgent,
} from '#/test-utils/team-agents'

describe('org tree', () => {
	it('starts at the head of the team and indents sub-agents', () => {
		const nodes = buildOrgTree(TEAM)
		expect(nodes[0]).toMatchObject({ id: 'a-chief', depth: 0, subCount: 2 })
		expect(nodes.find((n) => n.id === 'a-legal')).toMatchObject({
			depth: 1,
			subCount: 1,
		})
		expect(nodes.find((n) => n.id === 'a-registre')?.depth).toBe(2)
	})

	it('lists every agent once', () => {
		const nodes = buildOrgTree(TEAM)
		expect(new Set(nodes.map((n) => n.id)).size).toBe(TEAM.length)
	})

	it('treats an agent whose parent is missing as a root', () => {
		const nodes = buildOrgTree([REGISTRE])
		expect(nodes).toEqual([{ id: 'a-registre', depth: 0, subCount: 0 }])
	})

	it('survives a cycle', () => {
		const looped = [{ ...CHIEF, reports_to: 'a-dev' }, DEV]
		const nodes = buildOrgTree(looped)
		expect(new Set(nodes.map((n) => n.id)).size).toBe(nodes.length)
		expect(nodes).toHaveLength(2)
	})

	it('finds sub-agents and ancestors', () => {
		expect(subAgents(TEAM, 'a-chief').map((a) => a.id)).toEqual([
			'a-legal',
			'a-dev',
		])
		expect(ancestorsOf(TEAM, 'a-registre').map((a) => a.id)).toEqual([
			'a-legal',
			'a-chief',
		])
		expect(ancestorsOf(TEAM, 'a-chief')).toEqual([])
		expect(ancestorsOf([LEGAL, teamAgent('x', 'X')], 'x')).toEqual([])
	})
})
