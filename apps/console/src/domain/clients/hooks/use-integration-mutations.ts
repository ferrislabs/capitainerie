import { useMutation, useQueryClient } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { INTEGRATIONS_QUERY_KEY } from '#/domain/clients/hooks/use-integration-instances'
import { integrationErrorMessage } from '#/domain/clients/integration-errors'

function useRefresh() {
	const queryClient = useQueryClient()
	return () =>
		queryClient.invalidateQueries({ queryKey: INTEGRATIONS_QUERY_KEY })
}

export function useCreateIntegrationInstance() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/clients/{client_id}/integrations')
			.mutationOptions,
		gcTime: 0,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: integrationErrorMessage(mutation.error) }
}

export function useUpdateIntegrationInstance() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('patch', '/api/v1/integration-instances/{id}')
			.mutationOptions,
		gcTime: 0,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: integrationErrorMessage(mutation.error) }
}

export function useCheckIntegrationInstance() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('post', '/api/v1/integration-instances/{id}/check')
			.mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: integrationErrorMessage(mutation.error) }
}

export function useArchiveIntegrationInstance() {
	const refresh = useRefresh()
	const mutation = useMutation({
		...tanstackApi.mutation('delete', '/api/v1/integration-instances/{id}')
			.mutationOptions,
		onSuccess: refresh,
	})
	return { ...mutation, errorMessage: integrationErrorMessage(mutation.error) }
}
