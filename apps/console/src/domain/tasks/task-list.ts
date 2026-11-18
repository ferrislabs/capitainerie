import type { Task, TaskStatus } from '#/domain/tasks/model'

export const STATUS_ORDER: TaskStatus[] = [
	'progress',
	'todo',
	'backlog',
	'done',
	'cancelled',
]

export const BOARD_STATUSES: TaskStatus[] = STATUS_ORDER.slice(0, 4)

export const STATUS_LABEL: Record<TaskStatus, string> = {
	todo: 'À faire',
	progress: 'En cours',
	done: 'Terminé',
	cancelled: 'Rejeté',
	backlog: 'Proposé',
}

export interface TaskGroup {
	status: TaskStatus
	tasks: Task[]
}

export function groupByStatus(
	tasks: ReadonlyArray<Task>,
	options: { keepEmpty?: boolean; statuses?: TaskStatus[] } = {},
): TaskGroup[] {
	return (options.statuses ?? STATUS_ORDER)
		.map((status) => ({
			status,
			tasks: tasks.filter((t) => t.status === status),
		}))
		.filter((g) => options.keepEmpty || g.tasks.length > 0)
}

export function filterTasks(
	tasks: ReadonlyArray<Task>,
	query: string,
	agentName: (agentId: string | null) => string,
): Task[] {
	const q = query.trim().toLowerCase()
	if (!q) return [...tasks]
	return tasks.filter((t) =>
		`${t.id} ${t.title} ${t.project} ${agentName(t.agentId)}`
			.toLowerCase()
			.includes(q),
	)
}
