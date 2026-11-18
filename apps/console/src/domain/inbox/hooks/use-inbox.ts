import {
	type QueryClient,
	useInfiniteQuery,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { api, tanstackApi } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

const INBOX_PATH = '/api/v1/inbox'
const PAGE_SIZE = 50

export function invalidateInbox(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith(INBOX_PATH) ?? false
		},
	})
}

export function useInbox(unreadOnly: boolean) {
	const accessToken = useAccessToken()
	return useInfiniteQuery({
		queryKey: [
			{ _id: INBOX_PATH, infinite: true, query: { unread_only: unreadOnly } },
		],
		queryFn: ({ pageParam, signal }) =>
			api.get(INBOX_PATH, {
				query: {
					limit: PAGE_SIZE,
					...(unreadOnly ? { unread_only: true } : {}),
					...(pageParam ? { cursor: pageParam } : {}),
				},
				overrides: { signal },
			}),
		initialPageParam: '',
		getNextPageParam: (last: Schemas.InboxResponse) =>
			last.next_cursor ?? undefined,
		enabled: Boolean(accessToken),
	})
}

/** Nombre d'éléments non lus de la boîte de réception. */
export function useInboxUnread() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get(INBOX_PATH, {
			query: { unread_only: true, limit: 1 },
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useInboxUnreadCount(): number {
	return useInboxUnread().data?.unread ?? 0
}

export function useMarkInboxRead() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/inbox/read').mutationOptions,
		onSettled: () => invalidateInbox(client),
	})
}
