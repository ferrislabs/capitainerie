import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

function invalidateConnectors(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/mcp-connectors') ?? false
		},
	})
}

export function useMcpConnectors(archived = false) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/mcp-connectors', { query: { archived } })
			.queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useCreateMcpConnector() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/mcp-connectors').mutationOptions,
		onSuccess: () => invalidateConnectors(client),
	})
}

export function useUpdateMcpConnector() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/mcp-connectors/{connector_id}')
			.mutationOptions,
		onSuccess: () => invalidateConnectors(client),
	})
}

export function useAuthorizeMcpConnector() {
	return useMutation(
		tanstackApi.mutation(
			'post',
			'/api/v1/mcp-connectors/{connector_id}/authorize',
		).mutationOptions,
	)
}

export function useCompleteMcpOAuth() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/mcp-connectors/oauth/callback')
			.mutationOptions,
		onSuccess: () => invalidateConnectors(client),
	})
}

export function useDisconnectMcpConnector() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation(
			'delete',
			'/api/v1/mcp-connectors/{connector_id}/connection',
		).mutationOptions,
		onSuccess: () => invalidateConnectors(client),
	})
}

export function useTestMcpConnector() {
	return useMutation(
		tanstackApi.mutation('post', '/api/v1/mcp-connectors/{connector_id}/test')
			.mutationOptions,
	)
}
