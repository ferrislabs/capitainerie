import { describe, expect, it } from 'vitest'
import { currentTask, dashboardStats } from '#/domain/dashboard/stats'
import type { DemoAgent, DemoApproval, DemoTask } from '#/domain/demo'

const agent = (id: string, status: DemoAgent['status']) =>
	({ id, status }) as DemoAgent
const task = (id: string, agentId: string, status: DemoTask['status']) =>
	({ id, agentId, status }) as DemoTask
const approval = (status: DemoApproval['status']) =>
	({ status }) as DemoApproval

describe('dashboardStats', () => {
	it('counts running agents, open and blocked tasks, pending approvals', () => {
		const s = dashboardStats(
			[agent('a', 'running'), agent('b', 'idle')],
			[
				task('1', 'a', 'progress'),
				task('2', 'a', 'blocked'),
				task('3', 'a', 'done'),
				task('4', 'a', 'cancelled'),
			],
			[approval('pending'), approval('approved')],
		)
		expect(s).toEqual({ running: 1, open: 2, blocked: 1, pending: 1 })
	})
})

describe('currentTask', () => {
	it('returns the first open task of the agent', () => {
		const tasks = [
			task('1', 'a', 'done'),
			task('2', 'b', 'todo'),
			task('3', 'a', 'progress'),
			task('4', 'a', 'todo'),
		]
		expect(currentTask(tasks, 'a')?.id).toBe('3')
		expect(currentTask(tasks, 'z')).toBeUndefined()
	})
})
