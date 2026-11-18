import type { TaskStatus } from '#/domain/tasks/model'
import {
	STATUS_ORDER,
	type TaskFilters,
	type TaskGrouping,
	type TaskSort,
} from '#/domain/tasks/task-list'

export type TasksView = 'liste' | 'tableau'

export interface TasksSearch {
	vue: TasksView
	q: string
	statut?: TaskStatus
	agent?: string
	projet?: string
	tri?: TaskSort
	groupe?: TaskGrouping
}

export const DEFAULT_SORT: TaskSort = 'maj'
export const DEFAULT_GROUPING: TaskGrouping = 'statut'

const SORTS: ReadonlyArray<TaskSort> = ['maj', 'creation', 'titre', 'priorite']
const GROUPINGS: ReadonlyArray<TaskGrouping> = ['statut', 'agent', 'projet']

const oneOf = <T extends string>(
	values: ReadonlyArray<T>,
	value: unknown,
): T | undefined => values.find((v) => v === value)

const text = (value: unknown): string | undefined =>
	typeof value === 'string' && value !== '' ? value : undefined

export function parseTasksSearch(search: Record<string, unknown>): TasksSearch {
	const parsed: TasksSearch = {
		vue: search.vue === 'tableau' ? 'tableau' : 'liste',
		q: typeof search.q === 'string' ? search.q : '',
		statut: oneOf(STATUS_ORDER, search.statut),
		agent: text(search.agent),
		projet: text(search.projet),
		tri: oneOf(SORTS, search.tri),
		groupe: oneOf(GROUPINGS, search.groupe),
	}
	return Object.fromEntries(
		Object.entries(parsed).filter(([, v]) => v !== undefined),
	) as TasksSearch
}

export const filtersOf = (search: TasksSearch): TaskFilters => ({
	status: search.statut,
	agent: search.agent,
	project: search.projet,
})
