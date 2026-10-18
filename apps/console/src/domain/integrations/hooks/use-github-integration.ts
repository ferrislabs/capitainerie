import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'

export const GITHUB_QUERY_KEY = [
	{ _id: '/api/v1/integrations/github' },
] as const

export function useGithubIntegration(enabled = true) {
	return useQuery({
		...tanstackApi.get('/api/v1/integrations/github').queryOptions,
		enabled,
	})
}

export function useGithubRepositories(enabled: boolean) {
	return useQuery({
		...tanstackApi.get('/api/v1/integrations/github/repositories').queryOptions,
		enabled,
	})
}
