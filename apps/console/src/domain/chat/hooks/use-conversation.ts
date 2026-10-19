import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export function useConversation(id: string) {
	return useQuery(
		tanstackApi.get('/api/v1/conversations/{id}', { path: { id } })
			.queryOptions,
	)
}
