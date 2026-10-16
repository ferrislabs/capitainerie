import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { RUNS_QUERY_KEY } from '#/domain/runs/hooks/use-runs'

export function useSubmitTranscript() {
	const queryClient = useQueryClient()

	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/transcripts').mutationOptions,
		onSuccess: () =>
			queryClient.invalidateQueries({ queryKey: RUNS_QUERY_KEY }),
	})
}
