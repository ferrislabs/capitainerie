import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

export function invalidateApprovals(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/approvals') ?? false
		},
	})
}

export function useApprovals(status: Schemas.ApprovalStatusDto) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/approvals', { query: { status } }).queryOptions,
		enabled: Boolean(accessToken),
	})
}

/** Nombre d'approbations en attente pour toute l'organisation. */
export function usePendingApprovalsCount(): number {
	return useApprovals('pending').data?.pending ?? 0
}

export function useDecideApproval() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/approvals/{approval_id}/decide')
			.mutationOptions,
		onSettled: () => invalidateApprovals(client),
	})
}
