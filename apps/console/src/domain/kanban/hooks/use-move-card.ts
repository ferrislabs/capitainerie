import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { isCardQuery } from '#/domain/kanban/hooks/card-queries'
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
		onMutate: async (variables) => {
			await queryClient.cancelQueries({ queryKey: CARDS_QUERY_KEY })
			const previous =
				queryClient.getQueryData<Schemas.CardListResponse>(CARDS_QUERY_KEY)
			if (previous) {
				queryClient.setQueryData<Schemas.CardListResponse>(CARDS_QUERY_KEY, {
					...previous,
					items: previous.items.map((card) =>
						card.id === variables.path.card_id
							? { ...card, column: variables.body.to }
							: card,
					),
				})
			}
			return { previous }
		},
		onError: (error, _variables, context) => {
			if (context?.previous) {
				queryClient.setQueryData(CARDS_QUERY_KEY, context.previous)
			}
			if (isApiRequestError(error) && error.status === 409) {
				return queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY })
			}
		},
		onSuccess: () =>
			queryClient.invalidateQueries({
				predicate: (query) => isCardQuery(query.queryKey),
			}),
	})

	return {
		...mutation,
		errorMessage: moveErrorMessage(mutation.error),
		pendingCardId: mutation.isPending
			? mutation.variables?.path.card_id
			: undefined,
	}
}
