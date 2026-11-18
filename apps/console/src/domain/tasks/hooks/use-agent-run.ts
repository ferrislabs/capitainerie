import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'
import { isActiveRun, RUN_POLL_MS } from '#/domain/tasks/run-agent'

export function useRunCardAgent() {
	return useMutation(
		tanstackApi.mutation('post', '/api/v1/cards/{card_id}/agent-runs')
			.mutationOptions,
	)
}

export function useRunPolling(runId: string | null) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/runs/{run_id}', {
			path: { run_id: runId ?? '' },
		}).queryOptions,
		enabled: Boolean(accessToken) && runId !== null,
		refetchInterval: (query) => {
			const status = query.state.data?.status
			return status && !isActiveRun(status) ? false : RUN_POLL_MS
		},
	})
}

export function useRunTimeline(runId: string | null, enabled: boolean) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/runs/{run_id}/timeline', {
			path: { run_id: runId ?? '' },
		}).queryOptions,
		enabled: Boolean(accessToken) && runId !== null && enabled,
	})
}

export function useRefreshCard() {
	const client = useQueryClient()
	return useCallback(
		() =>
			client.invalidateQueries({
				predicate: (query) => {
					const [key] = query.queryKey as [{ _id?: string }?]
					return key?._id?.startsWith('/api/v1/cards') ?? false
				},
			}),
		[client],
	)
}
