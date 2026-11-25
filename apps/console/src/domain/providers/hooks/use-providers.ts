import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

function invalidateProviders(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/me/providers') ?? false
		},
	})
}

export function useMyProviders() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/me/providers').queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function usePutMyProvider() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('put', '/api/v1/me/providers/{kind}')
			.mutationOptions,
		onSuccess: () => invalidateProviders(client),
	})
}

export function useDeleteMyProvider() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('delete', '/api/v1/me/providers/{kind}')
			.mutationOptions,
		onSuccess: () => invalidateProviders(client),
	})
}

export function useTestMyProvider() {
	return useMutation(
		tanstackApi.mutation('post', '/api/v1/me/providers/{kind}/test')
			.mutationOptions,
	)
}

export function useOrgProviders() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/providers').queryOptions,
		enabled: Boolean(accessToken),
		retry: false,
	})
}
