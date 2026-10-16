import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export const CARDS_POLL_INTERVAL_MS = 3000
export const CARDS_QUERY_KEY = [{ _id: '/api/v1/cards' }] as const

export function useCards() {
	return useQuery({
		...tanstackApi.get('/api/v1/cards').queryOptions,
		refetchInterval: CARDS_POLL_INTERVAL_MS,
	})
}
