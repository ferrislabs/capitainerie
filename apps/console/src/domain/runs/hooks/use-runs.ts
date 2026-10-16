import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'

export const RUNS_PAGE_SIZE = 20
export const RUNS_POLL_INTERVAL_MS = 3000
export const RUNS_QUERY_KEY = [{ _id: '/api/v1/runs' }] as const

export function hasActiveRun(items: ReadonlyArray<Schemas.RunResponse>) {
	return items.some(
		(run) => run.status === 'queued' || run.status === 'running',
	)
}

export function useRuns() {
	return useQuery({
		...tanstackApi.get('/api/v1/runs', {
			query: { limit: RUNS_PAGE_SIZE, offset: 0 },
		}).queryOptions,
		refetchInterval: (query) =>
			query.state.data && hasActiveRun(query.state.data.items)
				? RUNS_POLL_INTERVAL_MS
				: false,
	})
}
