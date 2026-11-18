import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

function invalidatePath(client: QueryClient, path: string) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith(path) ?? false
		},
	})
}

export function useTeamAgents(archived = false) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/team/agents', { query: { archived } })
			.queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useTeamAgent(agentId: string) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/team/agents/{agent_id}', {
			path: { agent_id: agentId },
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useCreateTeamAgent() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/team/agents').mutationOptions,
		onSuccess: () => invalidatePath(client, '/api/v1/team/agents'),
	})
}

export function useUpdateTeamAgent() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/team/agents/{agent_id}')
			.mutationOptions,
		onSuccess: () => invalidatePath(client, '/api/v1/team/agents'),
	})
}
