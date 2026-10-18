import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import {
	executionErrorMessage,
	isVersionConflict,
} from '#/domain/kanban/execution-errors'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import { RUNS_QUERY_KEY } from '#/domain/runs/hooks/use-runs'

export function useExecuteCard() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards/{card_id}/executions')
			.mutationOptions,
		onSuccess: () =>
			Promise.all([
				queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY }),
				queryClient.invalidateQueries({ queryKey: RUNS_QUERY_KEY }),
			]),
		onError: (error) => {
			if (isVersionConflict(error)) {
				return queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY })
			}
		},
	})

	return {
		...mutation,
		errorMessage: executionErrorMessage(mutation.error),
	}
}
