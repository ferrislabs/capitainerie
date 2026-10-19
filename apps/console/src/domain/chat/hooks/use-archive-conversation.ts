import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { isConversationQuery } from '#/domain/chat/hooks/conversation-queries'

export function useArchiveConversation() {
	const queryClient = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/conversations/{id}')
			.mutationOptions,
		onSuccess: () =>
			queryClient.invalidateQueries({
				predicate: (query) => isConversationQuery(query.queryKey),
			}),
	})
}
