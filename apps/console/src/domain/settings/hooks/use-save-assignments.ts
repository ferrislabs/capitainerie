import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { mutationErrorMessage } from '#/domain/settings/errors'
import { ASSIGNMENTS_QUERY_KEY } from '#/domain/settings/hooks/use-settings-queries'
import { ADMIN_ONLY_MODELS } from '#/domain/settings/providers'
import type { ModelAssignments } from '#/domain/settings/roles'
import { isApiRequestError } from '#/lib/api-error'

export const EXECUTOR_MODEL_MESSAGE =
	'Le modèle de l’Exécutant doit être un modèle Claude (Anthropic).'

export function assignmentsErrorMessage(error: unknown): string | undefined {
	if (
		isApiRequestError(error) &&
		error.status === 422 &&
		/executor|claude/i.test(error.message)
	) {
		return EXECUTOR_MODEL_MESSAGE
	}
	return mutationErrorMessage(
		error,
		ADMIN_ONLY_MODELS,
		'Enregistrement impossible.',
	)
}

export function useSaveAssignments() {
	const queryClient = useQueryClient()
	const mutation = useMutation({
		...tanstackApi.mutation('put', '/api/v1/model-assignments').mutationOptions,
		onSuccess: (data) => {
			queryClient.setQueryData(ASSIGNMENTS_QUERY_KEY, data)
			return queryClient.invalidateQueries({ queryKey: ASSIGNMENTS_QUERY_KEY })
		},
	})

	return {
		save: (
			assignments: ModelAssignments,
			options?: { onSuccess?: () => void },
		) =>
			mutation.mutate(
				{ body: assignments },
				{ onSuccess: () => options?.onSuccess?.() },
			),
		reset: mutation.reset,
		isPending: mutation.isPending,
		isSuccess: mutation.isSuccess,
		errorMessage: assignmentsErrorMessage(mutation.error),
	}
}
