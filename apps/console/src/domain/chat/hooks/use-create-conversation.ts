import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { isConversationQuery } from '#/domain/chat/hooks/conversation-queries'

export function useCreateConversation() {
	const queryClient = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/conversations').mutationOptions,
		onSuccess: () =>
			queryClient.invalidateQueries({
				predicate: (query) => isConversationQuery(query.queryKey),
			}),
	})
}
