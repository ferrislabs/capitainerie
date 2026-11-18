import type { Schemas } from '#/api/api.client'
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

export type TaskSort = 'maj' | 'creation' | 'titre' | 'priorite'
export type TaskGrouping = 'statut' | 'agent' | 'projet'

export const SORT_LABEL: Record<TaskSort, string> = {
	maj: 'Mise à jour',
	creation: 'Création',
	titre: 'Titre',
	priorite: 'Priorité',
}

export const GROUPING_LABEL: Record<TaskGrouping, string> = {
	statut: 'Statut',
	agent: 'Agent',
	projet: 'Projet',
}

/** Valeur de filtre qui retient les tâches sans agent ou sans projet. */
export const NONE = 'aucun'

export interface TaskFilters {
	status?: TaskStatus
	agent?: string
	project?: string
}

export interface TaskGroup {
	key: string
	label: string
	status?: TaskStatus
	tasks: Task[]
}

export function groupByStatus(
	tasks: ReadonlyArray<Task>,
	options: { keepEmpty?: boolean; statuses?: TaskStatus[] } = {},
): TaskGroup[] {
	return (options.statuses ?? STATUS_ORDER)
		.map((status) => ({
			key: status,
			label: STATUS_LABEL[status],
			status,
			tasks: tasks.filter((t) => t.status === status),
		}))
		.filter((g) => options.keepEmpty || g.tasks.length > 0)
}

function groupByKey(
	tasks: ReadonlyArray<Task>,
	keyOf: (task: Task) => string | null,
	labelOf: (key: string, task: Task) => string,
	noneLabel: string,
): TaskGroup[] {
	const groups = new Map<string, TaskGroup>()
	for (const task of tasks) {
		const key = keyOf(task) ?? NONE
		const group = groups.get(key) ?? {
			key,
			label: key === NONE ? noneLabel : labelOf(key, task),
			tasks: [],
		}
		group.tasks.push(task)
		groups.set(key, group)
	}
	return [...groups.values()].sort((a, b) => {
		if (a.key === NONE) return 1
		if (b.key === NONE) return -1
		return a.label.localeCompare(b.label, 'fr')
	})
}

export function groupTasks(
	tasks: ReadonlyArray<Task>,
	grouping: TaskGrouping,
	options: {
		agentName: (agentId: string) => string
		keepEmpty?: boolean
		statuses?: TaskStatus[]
	},
): TaskGroup[] {
	switch (grouping) {
		case 'statut':
			return groupByStatus(tasks, options)
		case 'agent':
			return groupByKey(
				tasks,
				(t) => t.agentId,
				(id) => options.agentName(id),
				'Non assignée',
			)
		case 'projet':
			return groupByKey(
				tasks,
				(t) => t.projectId,
				(_, t) => t.project,
				'Sans projet',
			)
	}
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

const matches = (value: string | null, wanted: string | undefined) =>
	wanted === undefined || (value ?? NONE) === wanted

export function applyFilters(
	tasks: ReadonlyArray<Task>,
	filters: TaskFilters,
): Task[] {
	return tasks.filter(
		(t) =>
			(filters.status === undefined || t.status === filters.status) &&
			matches(t.agentId, filters.agent) &&
			matches(t.projectId, filters.project),
	)
}

export const activeFilterCount = (filters: TaskFilters): number =>
	[filters.status, filters.agent, filters.project].filter(
		(v) => v !== undefined,
	).length

const PRIORITY_RANK: Record<Schemas.PriorityDto, number> = {
	urgent: 0,
	high: 1,
	normal: 2,
	low: 3,
}

const newestFirst = (a: string, b: string) => b.localeCompare(a)

export function sortTasks(tasks: ReadonlyArray<Task>, sort: TaskSort): Task[] {
	const byUpdated = (a: Task, b: Task) =>
		newestFirst(a.card.updated_at, b.card.updated_at)
	const compare: Record<TaskSort, (a: Task, b: Task) => number> = {
		maj: byUpdated,
		creation: (a, b) => newestFirst(a.card.created_at, b.card.created_at),
		titre: (a, b) => a.title.localeCompare(b.title, 'fr'),
		priorite: (a, b) =>
			PRIORITY_RANK[a.card.priority] - PRIORITY_RANK[b.card.priority] ||
			byUpdated(a, b),
	}
	return [...tasks].sort(compare[sort])
}
