import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export function useGithubIntegration(enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/integrations/github').queryOptions,
		enabled: enabled && Boolean(accessToken),
	})
}

export function useGithubRepositories(enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/integrations/github/repositories').queryOptions,
		enabled: enabled && Boolean(accessToken),
		retry: false,
	})
}

export function useGithubIssues(repository: string) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/integrations/github/issues', {
			query: { repository },
		}).queryOptions,
		enabled: repository !== '' && Boolean(accessToken),
		retry: false,
	})
}
