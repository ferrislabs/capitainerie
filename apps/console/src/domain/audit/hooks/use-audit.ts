import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { useMemo } from 'react'
import { api, tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { monthWindow } from '#/domain/audit/model'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

const EVENTS_PATH = '/api/v1/audit/events'
const BUDGETS_PATH = '/api/v1/budgets'
const PAGE_SIZE = 50
const RUNS_LIMIT = 100

export type AuditEventFilters = Partial<{
	topic: string
	agent_id: string
	card_id: string
}>

export function useAuditEvents(filters: AuditEventFilters = {}) {
	const accessToken = useAccessToken()
	return useInfiniteQuery({
		queryKey: [{ _id: EVENTS_PATH, infinite: true, query: filters }],
		queryFn: ({ pageParam, signal }) =>
			api.get(EVENTS_PATH, {
				query: {
					...filters,
					limit: PAGE_SIZE,
					...(pageParam ? { cursor: pageParam } : {}),
				},
				overrides: { signal },
			}),
		initialPageParam: '',
		getNextPageParam: (last: Schemas.AuditEventListResponse) =>
			last.next_cursor ?? undefined,
		enabled: Boolean(accessToken),
	})
}

export interface RunFilters {
	status?: Schemas.RunStatusResponse
	agentId?: string
}

export function useAuditRuns({ status, agentId }: RunFilters) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/runs', {
			query: {
				limit: RUNS_LIMIT,
				...(status ? { status } : {}),
				...(agentId ? { agent_id: agentId } : {}),
			},
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useAgentCosts() {
	const accessToken = useAccessToken()
	const window = useMemo(() => monthWindow(new Date()), [])
	return useQuery({
		...tanstackApi.get('/api/v1/analytics/costs', {
			query: { by: 'agent', from: window.from, to: window.to },
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useBudgets() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get(BUDGETS_PATH).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useSaveBudgets() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('put', BUDGETS_PATH).mutationOptions,
		onSuccess: (data) => {
			client.setQueryData(tanstackApi.get(BUDGETS_PATH).queryKey, data)
		},
	})
}
