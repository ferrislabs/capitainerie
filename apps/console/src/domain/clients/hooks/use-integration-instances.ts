import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export const INTEGRATIONS_QUERY_KEY = [
	{ _id: '/api/v1/clients/{client_id}/integrations' },
] as const

export function useIntegrationInstances(clientId: string) {
	return useQuery(
		tanstackApi.get('/api/v1/clients/{client_id}/integrations', {
			path: { client_id: clientId },
			query: {},
		}).queryOptions,
	)
}
