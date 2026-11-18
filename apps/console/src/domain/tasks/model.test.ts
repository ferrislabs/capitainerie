import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	ancestorCount,
	childrenOf,
	columnOfStatus,
	statusOfColumn,
	toTask,
} from '#/domain/tasks/model'

const card = (
	id: string,
	extra: Partial<Schemas.CardResponse> = {},
): Schemas.CardResponse => ({
	id,
	column: 'todo',
	title: `Carte ${id}`,
	description: '',
	labels: [],
	acceptance_criteria: [],
	priority: 'normal',
	created_at: '2026-11-18T10:00:00Z',
	updated_at: '2026-11-18T10:00:00Z',
	created_by_subject: 'u',
	version: 4,
	...extra,
})

describe('column mapping', () => {
	it('round-trips every status', () => {
		for (const status of [
			'backlog',
			'todo',
			'progress',
			'done',
			'cancelled',
		] as const) {
			expect(statusOfColumn(columnOfStatus(status))).toBe(status)
		}
		expect(columnOfStatus('progress')).toBe('in_progress')
		expect(statusOfColumn('proposed')).toBe('backlog')
		expect(statusOfColumn('rejected')).toBe('cancelled')
	})
})

describe('toTask', () => {
	it('maps a card', () => {
		const task = toTask(
			card('0198a1b2-c3d4', {
				assignee_agent_id: 'ag1',
				description: ' Détails ',
				project: { id: 'p1', name: 'Site' },
				parent_id: 'par',
			}),
			Date.parse('2026-11-18T10:20:00Z'),
		)
		expect(task).toMatchObject({
			shortId: '0198a1b2',
			status: 'todo',
			agentId: 'ag1',
			project: 'Site',
			projectId: 'p1',
			note: 'Détails',
			parentId: 'par',
			age: 'il y a 20 min',
			version: 4,
		})
	})

	it('uses nulls for missing relations', () => {
		const task = toTask(card('x'))
		expect(task.agentId).toBeNull()
		expect(task.project).toBe('')
		expect(task.note).toBeNull()
	})
})

describe('hierarchy', () => {
	const tasks = [
		toTask(card('a')),
		toTask(card('b', { parent_id: 'a' })),
		toTask(card('c', { parent_id: 'b' })),
	]

	it('lists children', () => {
		expect(childrenOf(tasks, 'a').map((t) => t.id)).toEqual(['b'])
	})

	it('counts ancestors', () => {
		expect(ancestorCount(tasks, 'a')).toBe(0)
		expect(ancestorCount(tasks, 'c')).toBe(2)
	})
})
