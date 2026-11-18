import {
	type QueryClient,
	useMutation,
	useQuery,
	useQueryClient,
} from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { useAccessToken } from '#/domain/identity/hooks/use-access-token'

function invalidateCards(client: QueryClient) {
	return client.invalidateQueries({
		predicate: (query) => {
			const [key] = query.queryKey as [{ _id?: string }?]
			return key?._id?.startsWith('/api/v1/cards') ?? false
		},
	})
}

export function useCards() {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/cards', { query: {} }).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useCard(cardId: string) {
	const accessToken = useAccessToken()
	return useQuery({
		...tanstackApi.get('/api/v1/cards/{card_id}', {
			path: { card_id: cardId },
		}).queryOptions,
		enabled: Boolean(accessToken),
	})
}

export function useCreateCard() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards').mutationOptions,
		onSuccess: () => invalidateCards(client),
	})
}

export function useMoveCard() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards/{card_id}/move')
			.mutationOptions,
		onSettled: () => invalidateCards(client),
	})
}

export function useCommentCard() {
	const client = useQueryClient()
	return useMutation({
		...tanstackApi.mutation('post', '/api/v1/cards/{card_id}/comments')
			.mutationOptions,
		onSuccess: () => invalidateCards(client),
	})
}
