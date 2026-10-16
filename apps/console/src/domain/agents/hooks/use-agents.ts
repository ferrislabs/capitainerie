import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export function useAgents() {
	return useQuery(tanstackApi.get('/api/v1/agents').queryOptions)
}
