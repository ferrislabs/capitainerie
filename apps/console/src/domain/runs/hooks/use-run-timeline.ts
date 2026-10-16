import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'

export const TIMELINE_POLL_INTERVAL_MS = 2000

export function timelineRefetchInterval(
	data: Schemas.RunTimelineResponse | undefined,
): number | false {
	if (!data) return false
	const { status } = data.run
	return status === 'queued' || status === 'running'
		? TIMELINE_POLL_INTERVAL_MS
		: false
}

export function useRunTimeline(runId: string) {
	return useQuery({
		...tanstackApi.get('/api/v1/runs/{run_id}/timeline', {
			path: { run_id: runId },
		}).queryOptions,
		refetchInterval: (query) => timelineRefetchInterval(query.state.data),
	})
}
