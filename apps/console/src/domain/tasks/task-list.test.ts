import { describe, expect, it } from 'vitest'
import type { DemoTask } from '#/domain/demo'
import {
	filterTasks,
	groupByStatus,
	STATUS_ORDER,
} from '#/domain/tasks/task-list'

const task = (patch: Partial<DemoTask>): DemoTask => ({
	id: 'NAT-1',
	title: 'Titre',
	status: 'todo',
	agentId: 'registre',
	age: 'il y a 1 min',
	project: 'Projet',
	...patch,
})

const agentName = (id: string) => (id === 'registre' ? 'Registre' : id)

describe('groupByStatus', () => {
	it('orders groups and drops empty ones', () => {
		const groups = groupByStatus([
			task({ id: 'A', status: 'done' }),
			task({ id: 'B', status: 'blocked' }),
			task({ id: 'C', status: 'done' }),
		])
		expect(groups.map((g) => g.status)).toEqual(['blocked', 'done'])
		expect(groups[1]?.tasks.map((t) => t.id)).toEqual(['A', 'C'])
	})

	it('keeps empty groups on demand (board)', () => {
		const groups = groupByStatus([], { keepEmpty: true })
		expect(groups.map((g) => g.status)).toEqual(STATUS_ORDER)
	})
})

describe('filterTasks', () => {
	const tasks = [
		task({ id: 'NAT-691', title: 'Alpha', project: 'Veille' }),
		task({ id: 'NAT-2', title: 'Beta', project: 'Compta', agentId: 'x' }),
	]

	it('returns all on empty query', () => {
		expect(filterTasks(tasks, '  ', agentName)).toHaveLength(2)
	})

	it('matches id, title, project and agent', () => {
		expect(filterTasks(tasks, 'nat-691', agentName)[0]?.id).toBe('NAT-691')
		expect(filterTasks(tasks, 'beta', agentName)[0]?.id).toBe('NAT-2')
		expect(filterTasks(tasks, 'compta', agentName)[0]?.id).toBe('NAT-2')
		expect(filterTasks(tasks, 'registre', agentName)[0]?.id).toBe('NAT-691')
	})
})
