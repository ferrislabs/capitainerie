import { describe, expect, it } from 'vitest'
import {
	DEMO_AGENTS,
	DEMO_GRANTS,
	DEMO_PROJECTS,
	DEMO_TASKS,
} from '#/domain/demo'

const demoAgent = (id: string) => DEMO_AGENTS.find((agent) => agent.id === id)

describe('demo fixtures', () => {
	it('gives every agent a unique id', () => {
		const ids = DEMO_AGENTS.map((agent) => agent.id)
		expect(new Set(ids).size).toBe(ids.length)
	})

	it('grants connectors to known agents only', () => {
		for (const agentId of Object.keys(DEMO_GRANTS))
			expect(demoAgent(agentId), agentId).toBeDefined()
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
