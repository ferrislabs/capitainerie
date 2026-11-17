import { describe, expect, it } from 'vitest'
import {
	DEMO_AGENTS,
	DEMO_CONNECTORS,
	DEMO_GRANTS,
	DEMO_PROJECTS,
	DEMO_TASKS,
	demoAgent,
} from '#/domain/demo'

describe('demo fixtures', () => {
	it('gives every agent a unique id', () => {
		const ids = DEMO_AGENTS.map((agent) => agent.id)
		expect(new Set(ids).size).toBe(ids.length)
	})

	it('grants only connectors that exist and have an account', () => {
		const usable = new Set(
			DEMO_CONNECTORS.filter((c) => c.accounts.length > 0).map((c) => c.name),
		)
		for (const [agentId, names] of Object.entries(DEMO_GRANTS)) {
			expect(demoAgent(agentId), agentId).toBeDefined()
			for (const name of names) expect(usable.has(name), name).toBe(true)
		}
	})

	it('points every task and project to a known agent', () => {
		for (const task of DEMO_TASKS) expect(demoAgent(task.agentId)).toBeDefined()
		for (const project of DEMO_PROJECTS)
			expect(demoAgent(project.agentId)).toBeDefined()
	})

	it('links every sub-task to an existing parent', () => {
		const ids = new Set(DEMO_TASKS.map((task) => task.id))
		for (const task of DEMO_TASKS)
			if (task.parentId) expect(ids.has(task.parentId)).toBe(true)
	})

	it('keeps the reporting line inside the team', () => {
		const names = new Set(
			DEMO_AGENTS.map((a) => a.name.replace(/\s*\(.*\)/, '')),
		)
		for (const agent of DEMO_AGENTS)
			if (agent.reportsTo) expect(names.has(agent.reportsTo)).toBe(true)
	})
})
