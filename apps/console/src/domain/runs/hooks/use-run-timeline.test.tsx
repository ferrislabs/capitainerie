import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import {
	TIMELINE_POLL_INTERVAL_MS,
	timelineRefetchInterval,
	useRunTimeline,
} from '#/domain/runs/hooks/use-run-timeline'
import { SAFETY_REFETCH_MS } from '#/lib/realtime'
import { clearAuth, setAuth } from '#/store/auth.store'

function timeline(
	status: Schemas.RunStatusResponse,
): Schemas.RunTimelineResponse {
	return {
		run: {
			id: 'r',
			status,
			transcript_id: 't',
			transcript_title: 'Réunion',
			initiated_by: 'ada',
			pipeline: [],
			created_at: '2026-10-16T09:30:00Z',
		},
		steps: [],
		decisions: [],
		totals: { input_tokens: 0, output_tokens: 0, cost_micros: 0 },
	}
}

describe('timelineRefetchInterval', () => {
	it('polls while the run is queued or running', () => {
		expect(timelineRefetchInterval(timeline('queued'))).toBe(
			TIMELINE_POLL_INTERVAL_MS,
		)
		expect(timelineRefetchInterval(timeline('running'))).toBe(2000)
	})

	it('stops once the run is settled or unknown', () => {
		expect(timelineRefetchInterval(timeline('succeeded'))).toBe(false)
		expect(timelineRefetchInterval(timeline('failed'))).toBe(false)
		expect(timelineRefetchInterval(timeline('cancelled'))).toBe(false)
		expect(timelineRefetchInterval(undefined)).toBe(false)
	})

	it('slows down to the safety refetch while realtime is connected', () => {
		expect(timelineRefetchInterval(timeline('running'), 'connected')).toBe(
			SAFETY_REFETCH_MS,
		)
		expect(timelineRefetchInterval(timeline('succeeded'), 'connected')).toBe(
			false,
		)
	})

	it('keeps polling when realtime is degraded or offline', () => {
		expect(timelineRefetchInterval(timeline('running'), 'degraded')).toBe(
			TIMELINE_POLL_INTERVAL_MS,
		)
		expect(timelineRefetchInterval(timeline('queued'), 'offline')).toBe(
			TIMELINE_POLL_INTERVAL_MS,
		)
	})
})

describe('useRunTimeline', () => {
	const fetchMock = vi.fn()
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	})

	function wrapper({ children }: { children: ReactNode }) {
		return <QueryClientProvider client={client}>{children}</QueryClientProvider>
	}

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		client.clear()
	})

	it('fetches the timeline of the run', async () => {
		fetchMock.mockResolvedValue(
			new Response(JSON.stringify(timeline('succeeded')), {
				status: 200,
				headers: { 'Content-Type': 'application/json' },
			}),
		)
		const { result } = renderHook(() => useRunTimeline('r'), { wrapper })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url] = fetchMock.mock.calls[0] as [URL]
		expect(String(url)).toBe('http://api.test/api/v1/runs/r/timeline')
		expect(fetchMock).toHaveBeenCalledTimes(1)
	})
})
