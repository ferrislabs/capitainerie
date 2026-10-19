import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useRealtimeState } from '#/hooks/use-realtime'
import { type RealtimeState, SAFETY_REFETCH_MS } from '#/lib/realtime'

export const ACTIVE_RUNS_POLL_INTERVAL_MS = 5000

export function activeRunsRefetchInterval(
	realtime: RealtimeState = 'offline',
): number {
	return realtime === 'connected'
		? SAFETY_REFETCH_MS
		: ACTIVE_RUNS_POLL_INTERVAL_MS
}

export function useActiveRuns() {
	const realtime = useRealtimeState()
	return useQuery({
		...tanstackApi.get('/api/v1/runs/active').queryOptions,
		refetchInterval: activeRunsRefetchInterval(realtime),
	})
}

export function useActiveRunCount(): number | undefined {
	return useActiveRuns().data?.items.length
}
