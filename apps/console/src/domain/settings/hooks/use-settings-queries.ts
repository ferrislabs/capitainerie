import { type QueryClient, useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export const PROVIDERS_QUERY_KEY = [{ _id: '/api/v1/providers' }] as const
export const MODELS_QUERY_KEY = [{ _id: '/api/v1/models' }] as const
export const ASSIGNMENTS_QUERY_KEY = [
	{ _id: '/api/v1/model-assignments' },
] as const

export function useProviders() {
	return useQuery(tanstackApi.get('/api/v1/providers').queryOptions)
}

export function useModelCatalog() {
	return useQuery(tanstackApi.get('/api/v1/models').queryOptions)
}

export function useModelAssignments() {
	return useQuery(tanstackApi.get('/api/v1/model-assignments').queryOptions)
}

export function invalidateProviderData(queryClient: QueryClient) {
	return Promise.all([
		queryClient.invalidateQueries({ queryKey: PROVIDERS_QUERY_KEY }),
		queryClient.invalidateQueries({ queryKey: MODELS_QUERY_KEY }),
	])
}
