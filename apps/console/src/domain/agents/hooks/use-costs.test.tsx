import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { COSTS_WINDOW_DAYS, costsWindow } from '#/domain/agents/dimensions'
import { useAgents } from '#/domain/agents/hooks/use-agents'
import { useCosts } from '#/domain/agents/hooks/use-costs'
import { clearAuth, setAuth } from '#/store/auth.store'

const costs: Schemas.CostsResponse = {
	rows: [],
	total: { input_tokens: 0, output_tokens: 0, cost_micros: 0 },
}

function json(body: unknown) {
	return new Response(JSON.stringify(body), {
		status: 200,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('costsWindow', () => {
	it('spans the last thirty days', () => {
		const now = new Date('2026-10-17T12:00:00Z')
		const { from, to } = costsWindow(now)

		expect(to).toBe('2026-10-17T12:00:00.000Z')
		expect(Date.parse(to) - Date.parse(from)).toBe(
			COSTS_WINDOW_DAYS * 86_400_000,
		)
	})
})

describe('analytics hooks', () => {
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

	it('asks the API for the chosen grouping over the window', async () => {
		fetchMock.mockResolvedValue(json(costs))
		const { result } = renderHook(() => useCosts('user'), { wrapper })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))

		expect(result.current.data).toEqual(costs)
		const url = new URL(String(fetchMock.mock.calls[0]?.[0]))
		expect(url.pathname).toBe('/api/v1/analytics/costs')
		expect(url.searchParams.get('by')).toBe('user')
		expect(url.searchParams.get('from')).toBeTruthy()
		expect(url.searchParams.get('to')).toBeTruthy()
	})

	it('refetches when the grouping changes', async () => {
		fetchMock.mockImplementation(async () => json(costs))
		const { result, rerender } = renderHook(
			({ by }: { by: 'agent' | 'day' }) => useCosts(by),
			{ wrapper, initialProps: { by: 'agent' } },
		)
		await waitFor(() => expect(result.current.isSuccess).toBe(true))

		rerender({ by: 'day' })

		await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2))
		const url = new URL(String(fetchMock.mock.calls[1]?.[0]))
		expect(url.searchParams.get('by')).toBe('day')
	})

	it('lists the agents', async () => {
		fetchMock.mockResolvedValue(json([]))
		const { result } = renderHook(() => useAgents(), { wrapper })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))

		expect(result.current.data).toEqual([])
	})
})
