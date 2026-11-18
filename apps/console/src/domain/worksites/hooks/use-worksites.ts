import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'
import { launchErrorMessage } from '#/domain/worksites/worksite-errors'
import { hasActiveWorksite } from '#/domain/worksites/worksite-status'
import { SAFETY_REFETCH_MS } from '#/lib/realtime'

export const WORKSITES_POLL_INTERVAL_MS = 5000

const WORKSITES_PATH = '/api/v1/projects/{project_id}/worksites'

function invalidateAfterLaunch(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			const id = key?._id ?? ''
			return (
				id === WORKSITES_PATH ||
				id.startsWith('/api/v1/cards') ||
				id === '/api/v1/projects' ||
				id === '/api/v1/integrations/github/issues'
			)
		},
	})
}

export function useWorksites(projectId: string) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get(WORKSITES_PATH, { path: { project_id: projectId } })
			.queryOptions,
		enabled: projectId !== '' && Boolean(accessToken),
		refetchInterval: (query) =>
			hasActiveWorksite(query.state.data?.items)
				? WORKSITES_POLL_INTERVAL_MS
				: SAFETY_REFETCH_MS,
	})
}

export function useLaunchWorksites() {
	const client = useQueryClient()
	const mutation = useMutation({
		...tanstackApi.mutation('post', WORKSITES_PATH).mutationOptions,
		onSuccess: () => invalidateAfterLaunch(client),
	})
	return { ...mutation, errorMessage: launchErrorMessage(mutation.error) }
}
