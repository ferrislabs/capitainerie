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
			return key?._id === path
		},
	})
}

export function useProjects(archived: boolean) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/projects', { query: { archived } })
			.queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useClients() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/clients', { query: {} }).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useProjectCards() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/cards', { query: {} }).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useCreateProject() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/projects').mutationOptions,
		onSuccess: () => invalidatePath(client, '/api/v1/projects'),
	})
}

export function useUpdateProject() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('patch', '/api/v1/projects/{project_id}')
			.mutationOptions,
		onSuccess: () => invalidatePath(client, '/api/v1/projects'),
	})
}
