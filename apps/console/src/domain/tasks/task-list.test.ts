import { describe, expect, it } from 'vitest'
import type { Task } from '#/domain/tasks/model'
import {
	activeFilterCount,
	applyFilters,
	BOARD_STATUSES,
	filterTasks,
	groupByStatus,
	groupTasks,
	STATUS_ORDER,
	sortTasks,
} from '#/domain/tasks/task-list'
import { card } from '#/domain/tasks/test-fixtures'

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

describe('applyFilters', () => {
	const tasks = [
		task({ id: 'a', status: 'todo', agentId: 'a1', projectId: 'p1' }),
		task({ id: 'b', status: 'done', agentId: null, projectId: null }),
	]

	it('keeps everything without filters', () => {
		expect(applyFilters(tasks, {})).toHaveLength(2)
		expect(activeFilterCount({})).toBe(0)
	})

	it('filters by status, agent and project, "aucun" meaning none', () => {
		expect(applyFilters(tasks, { status: 'done' }).map((t) => t.id)).toEqual([
			'b',
		])
		expect(applyFilters(tasks, { agent: 'a1' }).map((t) => t.id)).toEqual(['a'])
		expect(applyFilters(tasks, { agent: 'aucun' }).map((t) => t.id)).toEqual([
			'b',
		])
		expect(
			applyFilters(tasks, { project: 'aucun', status: 'todo' }),
		).toHaveLength(0)
		expect(activeFilterCount({ status: 'todo', project: 'p1' })).toBe(2)
	})
})

describe('sortTasks', () => {
	const tasks = [
		task({
			id: 'a',
			title: 'Zeta',
			card: card('a', {
				priority: 'low',
				created_at: '2026-11-03T00:00:00Z',
				updated_at: '2026-11-01T00:00:00Z',
			}),
		}),
		task({
			id: 'b',
			title: 'alpha',
			card: card('b', {
				priority: 'urgent',
				created_at: '2026-11-01T00:00:00Z',
				updated_at: '2026-11-02T00:00:00Z',
			}),
		}),
	]
	const ids = (sort: Parameters<typeof sortTasks>[1]) =>
		sortTasks(tasks, sort).map((t) => t.id)

	it('sorts by update, creation, title and priority', () => {
		expect(ids('maj')).toEqual(['b', 'a'])
		expect(ids('creation')).toEqual(['a', 'b'])
		expect(ids('titre')).toEqual(['b', 'a'])
		expect(ids('priorite')).toEqual(['b', 'a'])
	})
})

describe('groupTasks', () => {
	const tasks = [
		task({ id: 'a', agentId: 'a2', projectId: 'p1', project: 'Web' }),
		task({ id: 'b', agentId: null, projectId: null, project: '' }),
		task({ id: 'c', agentId: 'a1', projectId: 'p2', project: 'Compta' }),
	]
	const agentName = (id: string) => (id === 'a1' ? 'Bravo' : 'Alpha')

	it('groups by agent, unassigned last', () => {
		expect(
			groupTasks(tasks, 'agent', { agentName }).map((g) => g.label),
		).toEqual(['Alpha', 'Bravo', 'Non assignée'])
	})

	it('groups by project, no project last', () => {
		expect(
			groupTasks(tasks, 'projet', { agentName }).map((g) => g.label),
		).toEqual(['Compta', 'Web', 'Sans projet'])
	})
})
