import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useRealtimeState } from '#/hooks/use-realtime'
import { type RealtimeState, SAFETY_REFETCH_MS } from '#/lib/realtime'

export const TIMELINE_POLL_INTERVAL_MS = 2000

export function timelineRefetchInterval(
	data: Schemas.RunTimelineResponse | undefined,
	realtime: RealtimeState = 'offline',
): number | false {
	if (!data) return false
	const { status } = data.run
	if (status !== 'queued' && status !== 'running') return false
	return realtime === 'connected'
		? SAFETY_REFETCH_MS
		: TIMELINE_POLL_INTERVAL_MS
}

export function useRunTimeline(runId: string) {
	const realtime = useRealtimeState()
	return useQuery({
		...tanstackApi.get('/api/v1/runs/{run_id}/timeline', {
			path: { run_id: runId },
		}).queryOptions,
		refetchInterval: (query) =>
			timelineRefetchInterval(query.state.data, realtime),
	})
}
