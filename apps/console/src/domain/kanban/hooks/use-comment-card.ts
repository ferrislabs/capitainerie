import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { isCardQuery } from '#/domain/kanban/hooks/card-queries'
import { isApiRequestError } from '#/lib/api-error'

export function commentErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (isApiRequestError(error)) {
		if (error.status === 404) return 'Cette carte n’existe plus.'
		if (error.status === 422) {
			return 'Le commentaire est vide ou dépasse 10 000 caractères.'
		}
	}
	return error instanceof Error ? error.message : 'Commentaire impossible.'
}

export function useCommentCard() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards/{card_id}/comments')
			.mutationOptions,
		onSuccess: () =>
			queryClient.invalidateQueries({
				predicate: (query) => isCardQuery(query.queryKey),
			}),
	})

	return {
		...mutation,
		errorMessage: commentErrorMessage(mutation.error),
	}
}
