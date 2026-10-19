import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { isCardQuery } from '#/domain/kanban/hooks/card-queries'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import { moveErrorMessage } from '#/domain/kanban/hooks/use-move-card'

export function usePatchCard() {
	const queryClient = useQueryClient()
	const refresh = () =>
		Promise.all([
			queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY }),
			queryClient.invalidateQueries({
				predicate: (query) => isCardQuery(query.queryKey),
			}),
		])
	const mutation = useMutation({
		...tanstackApi.mutation('patch', '/api/v1/cards/{card_id}').mutationOptions,
		onSuccess: refresh,
		onError: refresh,
	})
	return {
		...mutation,
		errorMessage: moveErrorMessage(mutation.error),
	}
}
