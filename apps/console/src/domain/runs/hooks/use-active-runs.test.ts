import { describe, expect, it } from 'vitest'
import {
	ACTIVE_RUNS_POLL_INTERVAL_MS,
	activeRunsRefetchInterval,
} from '#/domain/runs/hooks/use-active-runs'
import { SAFETY_REFETCH_MS } from '#/lib/realtime'

describe('activeRunsRefetchInterval', () => {
	it('polls every 5 seconds when realtime is not connected', () => {
		expect(activeRunsRefetchInterval('offline')).toBe(5000)
		expect(activeRunsRefetchInterval('degraded')).toBe(
			ACTIVE_RUNS_POLL_INTERVAL_MS,
		)
		expect(activeRunsRefetchInterval()).toBe(ACTIVE_RUNS_POLL_INTERVAL_MS)
	})

	it('only keeps the safety refetch while realtime is connected', () => {
		expect(activeRunsRefetchInterval('connected')).toBe(SAFETY_REFETCH_MS)
	})
})
