import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { directoryErrorMessage } from '#/domain/clients/errors'
import { isDirectoryQuery } from '#/domain/clients/hooks/directory-queries'
import { isCardQuery } from '#/domain/kanban/hooks/card-queries'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import { RUNS_QUERY_KEY } from '#/domain/runs/hooks/use-runs'

function useRefresh() {
	const queryClient = useQueryClient()
	return () =>
		Promise.all([
			queryClient.invalidateQueries({
				predicate: (query) => isDirectoryQuery(query.queryKey),
			}),
			queryClient.invalidateQueries({ queryKey: CARDS_QUERY_KEY }),
			queryClient.invalidateQueries({
				predicate: (query) => isCardQuery(query.queryKey),
			}),
			queryClient.invalidateQueries({ queryKey: RUNS_QUERY_KEY }),
		])
}

export function useCreateClient() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/clients').mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: directoryErrorMessage(mutation.error) }
}

export function useUpdateClient() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('patch', '/api/v1/clients/{client_id}')
			.mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: directoryErrorMessage(mutation.error) }
}

export function useCreateProject() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/projects').mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: directoryErrorMessage(mutation.error) }
}

export function useUpdateProject() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('patch', '/api/v1/projects/{project_id}')
			.mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: directoryErrorMessage(mutation.error) }
}
