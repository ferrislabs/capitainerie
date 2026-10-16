import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useRealtimeState } from '#/hooks/use-realtime'
import { type RealtimeState, SAFETY_REFETCH_MS } from '#/lib/realtime'

export const RUNS_PAGE_SIZE = 20
export const RUNS_POLL_INTERVAL_MS = 3000
export const RUNS_QUERY_KEY = [{ _id: '/api/v1/runs' }] as const

export function hasActiveRun(items: ReadonlyArray<Schemas.RunResponse>) {
	return items.some(
		(run) => run.status === 'queued' || run.status === 'running',
	)
}

export function runsRefetchInterval(
	data: Schemas.RunListResponse | undefined,
	realtime: RealtimeState = 'offline',
): number | false {
	if (realtime === 'connected') return SAFETY_REFETCH_MS
	return data && hasActiveRun(data.items) ? RUNS_POLL_INTERVAL_MS : false
}

export function useRuns() {
	const realtime = useRealtimeState()
	return useQuery({
		...tanstackApi.get('/api/v1/runs', {
			query: { limit: RUNS_PAGE_SIZE, offset: 0 },
		}).queryOptions,
		refetchInterval: (query) => runsRefetchInterval(query.state.data, realtime),
	})
}
