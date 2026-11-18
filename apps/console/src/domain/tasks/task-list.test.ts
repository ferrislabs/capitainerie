import { describe, expect, it } from 'vitest'
import type { Task } from '#/domain/tasks/model'
import {
	BOARD_STATUSES,
	filterTasks,
	groupByStatus,
	STATUS_ORDER,
} from '#/domain/tasks/task-list'

const task = (patch: Partial<Task>): Task =>
	({
		id: 'id-1',
		shortId: 'id-1',
		title: 'Titre',
		status: 'todo',
		agentId: 'a1',
		age: 'il y a 1 min',
		project: 'Projet',
		projectId: null,
		note: null,
		parentId: null,
		version: 1,
		...patch,
	}) as Task

const agentName = (id: string | null) => (id === 'a1' ? 'Registre' : 'Aucun')

describe('groupByStatus', () => {
	it('orders groups and drops empty ones', () => {
		const groups = groupByStatus([
			task({ id: 'A', status: 'done' }),
			task({ id: 'B', status: 'progress' }),
			task({ id: 'C', status: 'done' }),
		])
		expect(groups.map((g) => g.status)).toEqual(['progress', 'done'])
		expect(groups[1]?.tasks.map((t) => t.id)).toEqual(['A', 'C'])
	})

	it('keeps empty groups on demand (board)', () => {
		const groups = groupByStatus([], { keepEmpty: true })
		expect(groups.map((g) => g.status)).toEqual(STATUS_ORDER)
	})

	it('never offers statuses without a column', () => {
		expect(STATUS_ORDER).not.toContain('blocked')
		expect(STATUS_ORDER).not.toContain('review')
		expect(BOARD_STATUSES).toEqual(['progress', 'todo', 'backlog', 'done'])
	})
})

describe('filterTasks', () => {
	const tasks = [
		task({ id: 'abc', title: 'Alpha', project: 'Veille' }),
		task({ id: 'def', title: 'Beta', project: 'Compta', agentId: null }),
	]

	it('returns all on empty query', () => {
		expect(filterTasks(tasks, '  ', agentName)).toHaveLength(2)
	})

	it('matches id, title, project and agent', () => {
		expect(filterTasks(tasks, 'abc', agentName)[0]?.id).toBe('abc')
		expect(filterTasks(tasks, 'beta', agentName)[0]?.id).toBe('def')
		expect(filterTasks(tasks, 'compta', agentName)[0]?.id).toBe('def')
		expect(filterTasks(tasks, 'registre', agentName)[0]?.id).toBe('abc')
	})
})
