import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'
import type { Window } from '#/domain/metrics/model'

export interface MetricsQuery {
	window: Window
	agentId: string
	cardId: string
}

function params({ window, agentId, cardId }: MetricsQuery) {
	return {
		from: window.from,
		to: window.to,
		...(agentId ? { agent_id: agentId } : {}),
		...(cardId ? { card_id: cardId } : {}),
	}
}

export function useProjectCosts(query: MetricsQuery) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/analytics/costs/projects', {
			query: params(query),
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useAgentCostReport(query: MetricsQuery) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/analytics/costs/agents', {
			query: params(query),
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useTaskCosts(query: MetricsQuery) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/analytics/costs/tasks', {
			query: params(query),
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useUserCosts(query: MetricsQuery) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/analytics/costs/users', {
			query: params(query),
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}
