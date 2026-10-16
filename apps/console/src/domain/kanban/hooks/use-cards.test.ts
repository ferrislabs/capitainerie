import { describe, expect, it } from 'vitest'
import {
	CARDS_POLL_INTERVAL_MS,
	cardsRefetchInterval,
} from '#/domain/kanban/hooks/use-cards'
import { SAFETY_REFETCH_MS } from '#/lib/realtime'

describe('cardsRefetchInterval', () => {
	it('keeps polling when realtime is degraded or offline', () => {
		expect(cardsRefetchInterval('degraded')).toBe(CARDS_POLL_INTERVAL_MS)
		expect(cardsRefetchInterval('offline')).toBe(CARDS_POLL_INTERVAL_MS)
		expect(cardsRefetchInterval()).toBe(CARDS_POLL_INTERVAL_MS)
	})

	it('only keeps the safety refetch when realtime is connected', () => {
		expect(cardsRefetchInterval('connected')).toBe(SAFETY_REFETCH_MS)
	})
})
