import {
	keepPreviousData,
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export function invalidateSources(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/sources') ?? false
		},
	})
}

export function useSources(archived = false, enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/sources', { query: { archived } }).queryOptions,
		enabled: Boolean(accessToken) && enabled,
	})
}

export function useSource(sourceId: string, enabled = true) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/sources/{source_id}', {
			path: { source_id: sourceId },
		}).queryOptions,
		enabled: Boolean(accessToken) && Boolean(sourceId) && enabled,
	})
}

export function useSourceEvents(
	sourceId: string,
	limit: number,
	offset: number,
) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/sources/{source_id}/events', {
			path: { source_id: sourceId },
			query: { limit, offset },
		}).queryOptions,
		enabled: Boolean(accessToken),
		placeholderData: keepPreviousData,
	})
}

export function useCreateSource() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/sources').mutationOptions,
		onSuccess: () => invalidateSources(client),
	})
}

export function useUpdateSource() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/sources/{source_id}')
			.mutationOptions,
		onSuccess: () => invalidateSources(client),
	})
}

export function useArchiveSource() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/sources/{source_id}/archive')
			.mutationOptions,
		onSuccess: () => invalidateSources(client),
	})
}

export function useRotateSourceToken() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/sources/{source_id}/rotate-token')
			.mutationOptions,
		onSuccess: () => invalidateSources(client),
	})
}

export function useTestSource() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/sources/{source_id}/test')
			.mutationOptions,
		onSuccess: () => invalidateSources(client),
	})
}
