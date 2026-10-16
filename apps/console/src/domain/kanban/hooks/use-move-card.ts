import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import { isApiRequestError } from '#/lib/api-error'

export const CONFLICT_MESSAGE = 'La carte a changé, rechargée.'

export function moveErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (isApiRequestError(error) && error.status === 409) return CONFLICT_MESSAGE
	return error instanceof Error ? error.message : 'Déplacement impossible.'
}

export function useMoveCard() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards/{card_id}/move')
			.mutationOptions,
		onSuccess: () =>
			queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY }),
		onError: (error) => {
			if (isApiRequestError(error) && error.status === 409) {
				return queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY })
			}
		},
	})

	return {
		...mutation,
		errorMessage: moveErrorMessage(mutation.error),
		pendingCardId: mutation.isPending
			? mutation.variables?.path.card_id
			: undefined,
	}
}
