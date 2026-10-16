import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	RUNS_POLL_INTERVAL_MS,
	runsRefetchInterval,
} from '#/domain/runs/hooks/use-runs'
import { SAFETY_REFETCH_MS } from '#/lib/realtime'

function list(status: Schemas.RunStatusResponse): Schemas.RunListResponse {
	return {
		items: [
			{
				id: 'r',
				status,
				transcript_id: 't',
				transcript_title: 'Réunion',
				initiated_by: 'ada',
				pipeline: [],
				created_at: '2026-10-16T09:30:00Z',
			},
		],
		total: 1,
	}
}

describe('runsRefetchInterval', () => {
	it('polls while a run is active and realtime is not connected', () => {
		expect(runsRefetchInterval(list('running'), 'degraded')).toBe(
			RUNS_POLL_INTERVAL_MS,
		)
		expect(runsRefetchInterval(list('queued'), 'offline')).toBe(
			RUNS_POLL_INTERVAL_MS,
		)
		expect(runsRefetchInterval(list('running'))).toBe(RUNS_POLL_INTERVAL_MS)
	})

	it('stops polling when nothing is active and realtime is not connected', () => {
		expect(runsRefetchInterval(list('succeeded'), 'degraded')).toBe(false)
		expect(runsRefetchInterval(undefined, 'offline')).toBe(false)
	})

	it('only keeps the safety refetch while realtime is connected', () => {
		expect(runsRefetchInterval(list('running'), 'connected')).toBe(
			SAFETY_REFETCH_MS,
		)
		expect(runsRefetchInterval(undefined, 'connected')).toBe(SAFETY_REFETCH_MS)
	})
})
