import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import { CONFLICT_MESSAGE } from '#/domain/kanban/hooks/use-move-card'
import { usePatchCard } from '#/domain/kanban/hooks/use-patch-card'
import { clearAuth, setAuth } from '#/store/auth.store'

function json(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('usePatchCard', () => {
	const fetchMock = vi.fn()
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	})

	function wrapper({ children }: { children: ReactNode }) {
		return <QueryClientProvider client={client}>{children}</QueryClientProvider>
	}

	function invalidated() {
		return client.getQueryState(CARDS_QUERY_KEY)?.isInvalidated
	}

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		client.setQueryData(CARDS_QUERY_KEY, { items: [] })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		client.clear()
	})

	it('patches the links with the expected version and refreshes the board', async () => {
		fetchMock.mockResolvedValue(json(200, { id: 'c1' }))
		const { result } = renderHook(() => usePatchCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { expected_version: 3, client_id: null, project_id: 'p1' },
		})

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/cards/c1')
		expect(init.method).toBe('PATCH')
		expect(JSON.parse(String(init.body))).toEqual({
			expected_version: 3,
			client_id: null,
			project_id: 'p1',
		})
		expect(invalidated()).toBe(true)
	})

	it('reloads and says so on a stale version', async () => {
		fetchMock.mockResolvedValue(
			json(409, { code: 'conflict', status: 409, message: 'stale' }),
		)
		const { result } = renderHook(() => usePatchCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { expected_version: 1, client_id: 'acme' },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toBe(CONFLICT_MESSAGE)
		expect(invalidated()).toBe(true)
	})
})
