import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useRealtimeState } from '#/hooks/use-realtime'
import { type RealtimeState, SAFETY_REFETCH_MS } from '#/lib/realtime'

export const CARDS_POLL_INTERVAL_MS = 3000
export const CARDS_QUERY_KEY = [{ _id: '/api/v1/cards' }] as const

export function cardsRefetchInterval(realtime: RealtimeState = 'offline') {
	return realtime === 'connected' ? SAFETY_REFETCH_MS : CARDS_POLL_INTERVAL_MS
}

export function useCards() {
	const realtime = useRealtimeState()
	return useQuery({
		...tanstackApi.get('/api/v1/cards').queryOptions,
		refetchInterval: cardsRefetchInterval(realtime),
	})
}
