import type { Schemas } from '#/api/api.client'
import { relativeAge } from '#/domain/dashboard/format'
import type { DemoTaskStatus } from '#/domain/demo'
import { shortTaskId } from '#/domain/tasks/task-ids'

export type TaskStatus = Extract<
	DemoTaskStatus,
	'backlog' | 'todo' | 'progress' | 'done' | 'cancelled'
>

const STATUS_OF_COLUMN: Record<Schemas.ColumnDto, TaskStatus> = {
	proposed: 'backlog',
	todo: 'todo',
	in_progress: 'progress',
	done: 'done',
	rejected: 'cancelled',
}

const COLUMN_OF_STATUS: Record<TaskStatus, Schemas.ColumnDto> = {
	backlog: 'proposed',
	todo: 'todo',
	progress: 'in_progress',
	done: 'done',
	cancelled: 'rejected',
}

export const statusOfColumn = (column: Schemas.ColumnDto): TaskStatus =>
	STATUS_OF_COLUMN[column]

export const columnOfStatus = (status: TaskStatus): Schemas.ColumnDto =>
	COLUMN_OF_STATUS[status]

export interface Task {
	id: string
	shortId: string
	title: string
	status: TaskStatus
	agentId: string | null
	age: string
	project: string
	projectId: string | null
	note: string | null
	parentId: string | null
	version: number
	card: Schemas.CardResponse
}

export function toTask(
	card: Schemas.CardResponse,
	now: number = Date.now(),
): Task {
	return {
		id: card.id,
		shortId: shortTaskId(card.id),
		title: card.title,
		status: statusOfColumn(card.column),
		agentId: card.assignee_agent_id ?? null,
		age: relativeAge(card.updated_at, now),
		project: card.project?.name ?? '',
		projectId: card.project?.id ?? null,
		note: card.description.trim() || null,
		parentId: card.parent_id ?? null,
		version: card.version,
		card,
	}
}

export function childrenOf(tasks: ReadonlyArray<Task>, taskId: string): Task[] {
	return tasks.filter((t) => t.parentId === taskId)
}

export function ancestorCount(
	tasks: ReadonlyArray<Task>,
	taskId: string,
): number {
	const byId = new Map(tasks.map((t) => [t.id, t]))
	let count = 0
	let parent = byId.get(taskId)?.parentId
	while (parent && count < 10) {
		count += 1
		parent = byId.get(parent)?.parentId
	}
	return count
}

export const PRIORITY_LABEL: Record<Schemas.PriorityDto, string> = {
	low: 'Basse',
	normal: 'Normale',
	high: 'Haute',
	urgent: 'Urgente',
}
