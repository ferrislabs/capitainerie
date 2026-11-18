import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import { tanstackApi } from '#/api'
import { dayWindow } from '#/domain/dashboard/chart'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

const LIVE_REFRESH_MS = 15_000
const RECENT_RUNS = 20

export function useActiveRuns() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/runs/active').queryOptions,
		enabled: Boolean(accessToken),
		refetchInterval: LIVE_REFRESH_MS,
	})
}

export function useRecentRuns() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/runs', { query: { limit: RECENT_RUNS } })
			.queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useDailyCosts() {
	const accessToken = useAccessToken()
	const window = useMemo(() => dayWindow(new Date()), [])
	const query = useQuery({
		...tanstackApi.get('/api/v1/analytics/costs', {
			query: { by: 'day', from: window.from, to: window.to },
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
	return { query, days: window.days }
}

export function useDashboardCards() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/cards', { query: {} }).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useAgentAnalytics() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/team/agents', { query: { archived: false } })
			.queryOptions,
		select: (data) => data.items,
		enabled: Boolean(accessToken),
	})
}
