import { type Query, useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import {
	restoreFailure,
	revertErrorMessage,
} from '#/domain/integrations/journal/revert-errors'
import { isApiRequestError } from '#/lib/api-error'

const JOURNAL_QUERY_IDS: ReadonlyArray<string> = [
	'/api/v1/cards/{card_id}/integration-actions',
	'/api/v1/runs/{run_id}/integration-actions',
]

function isJournalQuery(query: Query): boolean {
	const head: unknown = query.queryKey[0]
	if (typeof head !== 'object' || head === null || !('_id' in head)) {
		return false
	}
	const id = (head as { _id: unknown })._id
	return typeof id === 'string' && JOURNAL_QUERY_IDS.includes(id)
}

export function useRevertIntegrationAction() {
	const queryClient = useQueryClient()
	const reloadJournals = () =>
		queryClient.invalidateQueries({ predicate: isJournalQuery })
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/integration-actions/{id}/revert')
			.mutationOptions,
		onSuccess: () => {
			void reloadJournals()
		},
		onError: (error) => {
			if (isApiRequestError(error) && error.status === 409) {
				void reloadJournals()
			}
		},
	})
	const failure = mutation.data
		? restoreFailure(mutation.data.items)
		: undefined

	return {
		...mutation,
		errorMessage: revertErrorMessage(mutation.error) ?? failure,
		restored: mutation.isSuccess && failure === undefined,
	}
}
