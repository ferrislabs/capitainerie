import type { DemoTask, DemoTaskStatus } from '#/domain/demo'

export const STATUS_ORDER: DemoTaskStatus[] = [
	'blocked',
	'progress',
	'review',
	'todo',
	'backlog',
	'done',
	'cancelled',
]

export const BOARD_STATUSES: DemoTaskStatus[] = STATUS_ORDER.slice(0, 6)

export const STATUS_LABEL: Record<DemoTaskStatus, string> = {
	todo: 'À faire',
	progress: 'En cours',
	review: 'En revue',
	done: 'Terminé',
	blocked: 'Bloqué',
	cancelled: 'Annulé',
	backlog: 'Backlog',
}

export interface TaskGroup {
	status: DemoTaskStatus
	tasks: DemoTask[]
}

export function groupByStatus(
	tasks: ReadonlyArray<DemoTask>,
	options: { keepEmpty?: boolean; statuses?: DemoTaskStatus[] } = {},
): TaskGroup[] {
	return (options.statuses ?? STATUS_ORDER)
		.map((status) => ({
			status,
			tasks: tasks.filter((t) => t.status === status),
		}))
		.filter((g) => options.keepEmpty || g.tasks.length > 0)
}

export function filterTasks(
	tasks: ReadonlyArray<DemoTask>,
	query: string,
	agentName: (agentId: string) => string,
): DemoTask[] {
	const q = query.trim().toLowerCase()
	if (!q) return [...tasks]
	return tasks.filter((t) =>
		`${t.id} ${t.title} ${t.project} ${agentName(t.agentId)}`
			.toLowerCase()
			.includes(q),
	)
}
