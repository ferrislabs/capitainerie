import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { CARDS_QUERY_KEY } from '#/domain/kanban/hooks/use-cards'
import {
	CONFLICT_MESSAGE,
	useMoveCard,
} from '#/domain/kanban/hooks/use-move-card'
import { clearAuth, setAuth } from '#/store/auth.store'

function json(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('useMoveCard', () => {
	const fetchMock = vi.fn()
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	})

	function wrapper({ children }: { children: ReactNode }) {
		return <QueryClientProvider client={client}>{children}</QueryClientProvider>
	}

	function seed() {
		client.setQueryData([{ _id: '/api/v1/cards' }], { items: [] })
	}

	function invalidated() {
		const query = client
			.getQueryCache()
			.findAll({ queryKey: CARDS_QUERY_KEY })[0]
		return client.getQueryState(query.queryKey)?.isInvalidated
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

	it('posts the move and invalidates the cards list', async () => {
		fetchMock.mockResolvedValue(json(200, { id: 'c1' }))
		seed()
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'todo', expected_version: 3 },
		})

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/cards/c1/move')
		expect(init.method).toBe('POST')
		expect(JSON.parse(String(init.body))).toEqual({
			to: 'todo',
			expected_version: 3,
		})
		expect(invalidated()).toBe(true)
	})

	it('surfaces the conflict message and invalidates on 409', async () => {
		fetchMock.mockResolvedValue(
			json(409, { code: 'conflict', status: 409, message: 'stale' }),
		)
		seed()
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'done', expected_version: 1 },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toBe(CONFLICT_MESSAGE)
		await waitFor(() => expect(invalidated()).toBe(true))
	})

	it('surfaces the server message on 403', async () => {
		fetchMock.mockResolvedValue(
			json(403, { code: 'forbidden', status: 403, message: 'Interdit' }),
		)
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'done', expected_version: 1 },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toBe('Interdit')
	})

	function card(id: string, column: string) {
		return { id, column, version: 3 }
	}

	function cachedColumns() {
		const data = client.getQueryData<{
			items: Array<{ id: string; column: string }>
		}>(CARDS_QUERY_KEY)
		return data?.items.map((item) => `${item.id}:${item.column}`)
	}

	it('moves the card in the cache before the server answers', async () => {
		let answer: (response: Response) => void = () => {}
		fetchMock.mockReturnValue(
			new Promise<Response>((resolve) => {
				answer = resolve
			}),
		)
		client.setQueryData(CARDS_QUERY_KEY, {
			items: [card('c1', 'todo'), card('c2', 'todo')],
		})
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'in_progress', expected_version: 3 },
		})

		await waitFor(() =>
			expect(cachedColumns()).toEqual(['c1:in_progress', 'c2:todo']),
		)
		expect(result.current.pendingCardId).toBe('c1')
		answer(json(200, { id: 'c1' }))
		await waitFor(() => expect(result.current.isSuccess).toBe(true))
	})

	it('puts the card back and reloads the board on 409', async () => {
		fetchMock.mockResolvedValue(
			json(409, { code: 'conflict', status: 409, message: 'stale' }),
		)
		client.setQueryData(CARDS_QUERY_KEY, {
			items: [card('c1', 'todo'), card('c2', 'done')],
		})
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'done', expected_version: 1 },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(cachedColumns()).toEqual(['c1:todo', 'c2:done'])
		await waitFor(() => expect(invalidated()).toBe(true))
		expect(result.current.errorMessage).toBe(CONFLICT_MESSAGE)
	})

	it('puts the card back and keeps the message on another error', async () => {
		fetchMock.mockResolvedValue(
			json(403, { code: 'forbidden', status: 403, message: 'Interdit' }),
		)
		client.setQueryData(CARDS_QUERY_KEY, { items: [card('c1', 'proposed')] })
		const { result } = renderHook(() => useMoveCard(), { wrapper })

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { to: 'done', expected_version: 3 },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(cachedColumns()).toEqual(['c1:proposed'])
		expect(result.current.errorMessage).toBe('Interdit')
		await waitFor(() => expect(invalidated()).toBe(true))
	})
})
