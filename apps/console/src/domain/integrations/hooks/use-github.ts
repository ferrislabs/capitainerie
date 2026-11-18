import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { api, tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'
import { hasErrorKey } from '#/lib/api-error'

function invalidateGithub(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/integrations/github') ?? false
		},
	})
}

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

export function useCreateGithubManifest() {
	return useMutation(
		tanstackApi.mutation('post', '/api/v1/integrations/github/app/manifest')
			.mutationOptions,
	)
}

export function useCreateGithubApp() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/integrations/github/app')
			.mutationOptions,
		onSuccess: () => invalidateGithub(client),
	})
}

export function useDeleteGithubApp() {
	const client = useQueryClient()
	return useMutation({
		mutationFn: () => api.delete('/api/v1/integrations/github/app'),
		onSuccess: () => invalidateGithub(client),
	})
}

export function useGithubInstallUrl() {
	return useMutation({
		mutationFn: () => api.get('/api/v1/integrations/github/install'),
	})
}

export function useConnectGithub() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/integrations/github/callback')
			.mutationOptions,
		onSuccess: () => invalidateGithub(client),
	})
}

export function useDisconnectGithub() {
	const client = useQueryClient()
	return useMutation({
		mutationFn: () => api.delete('/api/v1/integrations/github'),
		onSuccess: () => invalidateGithub(client),
	})
}

export type RepositoriesState =
	| { kind: 'loading' }
	| { kind: 'not_connected' }
	| { kind: 'error' }
	| {
			kind: 'ready'
			repositories: ReadonlyArray<Schemas.RepositoryResponse>
	  }

/** Les dépôts que l'installation GitHub de l'organisation atteint, ou pourquoi il n'y en a pas. */
export function useGithubRepositoryChoices(enabled = true): RepositoriesState {
	const integration = useGithubIntegration(enabled)
	const installation = integration.data?.installation ?? null
	const repositories = useGithubRepositories(enabled && installation !== null)
	if (integration.isError) return { kind: 'error' }
	if (integration.data === undefined) return { kind: 'loading' }
	if (
		installation === null ||
		hasErrorKey(repositories.error, 'github_not_connected')
	) {
		return { kind: 'not_connected' }
	}
	if (repositories.isError) return { kind: 'error' }
	if (repositories.data === undefined) return { kind: 'loading' }
	return { kind: 'ready', repositories: repositories.data.repositories }
}
