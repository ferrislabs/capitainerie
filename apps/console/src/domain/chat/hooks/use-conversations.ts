import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export function useConversations() {
	return useQuery(tanstackApi.get('/api/v1/conversations').queryOptions)
}
