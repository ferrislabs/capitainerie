export type TasksView = 'liste' | 'tableau'

export interface TasksSearch {
	vue: TasksView
	q: string
}

export function parseTasksSearch(search: Record<string, unknown>): TasksSearch {
	return {
		vue: search.vue === 'tableau' ? 'tableau' : 'liste',
		q: typeof search.q === 'string' ? search.q : '',
	}
}
