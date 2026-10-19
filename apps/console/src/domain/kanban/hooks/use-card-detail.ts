import { useQuery } from '@tanstack/react-query'
import { tanstackApi } from '#/api'
import { cardsRefetchInterval } from '#/domain/kanban/hooks/use-cards'
import { useRealtimeState } from '#/hooks/use-realtime'

export function useCardDetail(cardId: string | undefined) {
	const realtime = useRealtimeState()
	return useQuery({
		...tanstackApi.get('/api/v1/cards/{card_id}', {
			path: { card_id: cardId ?? '' },
		}).queryOptions,
		enabled: cardId !== undefined,
		refetchInterval: cardsRefetchInterval(realtime),
	})
}
