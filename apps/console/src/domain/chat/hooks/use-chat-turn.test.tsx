import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useChatTurn } from '#/domain/chat/hooks/use-chat-turn'
import { openSse, sseEvent, sseResponse } from '#/domain/chat/test-utils'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

describe('useChatTurn', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	function setup() {
		return renderHook(() => useChatTurn('c1'), { wrapper: harness.wrapper })
	}

	it('posts the message with the bearer token and folds the events into the draft', async () => {
		harness.seed('/api/v1/conversations/{id}', { messages: [] })
		const answered = sseResponse([
			sseEvent('delta', { text: 'Je cherche.' }),
			sseEvent('tool', {
				phase: 'start',
				id: 't1',
				name: 'search_cards',
				arguments: '{}',
			}),
			sseEvent('tool', {
				phase: 'end',
				id: 't1',
				name: 'search_cards',
				succeeded: true,
				latency_ms: 5,
				summary: '2 cartes trouvées',
			}),
			sseEvent('card', { card_id: 'k1', title: 'README' }),
			sseEvent('done', {
				status: 'complete',
				usage: { input_tokens: 1, output_tokens: 2 },
			}),
		])
		fetchMock.mockResolvedValue(answered)
		const { result } = setup()

		await act(async () => {
			await result.current.send('Ajoute un README')
		})

		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
		expect(url).toBe('http://api.test/api/v1/conversations/c1/messages')
		expect(init.method).toBe('POST')
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
		expect(new Headers(init.headers).get('Content-Type')).toBe(
			'application/json',
		)
		expect(JSON.parse(String(init.body))).toEqual({
			message: 'Ajoute un README',
		})
		expect(result.current.state.status).toBe('idle')
		expect(result.current.state.error).toBeUndefined()
		expect(
			harness.client.getQueryState([{ _id: '/api/v1/conversations/{id}' }])
				?.isInvalidated,
		).toBe(true)
	})

	it('sends an empty body to answer the last message', async () => {
		fetchMock.mockResolvedValue(sseResponse([]))
		const { result } = setup()

		await act(async () => {
			await result.current.send()
		})

		const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
		expect(JSON.parse(String(init.body))).toEqual({})
	})

	it('streams the draft while the turn runs and clears it when it ends', async () => {
		let feed: ReturnType<typeof openSse> | undefined
		fetchMock.mockImplementation((_url: string, init: RequestInit) => {
			feed = openSse(init.signal as AbortSignal)
			return Promise.resolve(feed.response)
		})
		const { result } = setup()

		let pending: Promise<void> | undefined
		act(() => {
			pending = result.current.send('Salut')
		})
		await waitFor(() => expect(feed).toBeDefined())
		expect(result.current.state.status).toBe('streaming')
		expect(result.current.state.pendingMessage).toBe('Salut')

		await act(async () => {
			feed?.push(sseEvent('delta', { text: 'Bon' }))
		})
		await waitFor(() => expect(result.current.state.draft.text).toBe('Bon'))
		await act(async () => {
			feed?.push(sseEvent('delta', { text: 'jour' }))
		})
		await waitFor(() => expect(result.current.state.draft.text).toBe('Bonjour'))

		await act(async () => {
			feed?.close()
			await pending
		})
		expect(result.current.state.status).toBe('idle')
		expect(result.current.state.draft.text).toBe('')
	})

	it('aborts the request on abort and ends quietly', async () => {
		let signal: AbortSignal | undefined
		let feed: ReturnType<typeof openSse> | undefined
		fetchMock.mockImplementation((_url: string, init: RequestInit) => {
			signal = init.signal as AbortSignal
			feed = openSse(signal)
			return Promise.resolve(feed.response)
		})
		const { result } = setup()
		let pending: Promise<void> | undefined
		act(() => {
			pending = result.current.send('Salut')
		})
		await waitFor(() => expect(signal).toBeDefined())
		await act(async () => {
			feed?.push(sseEvent('delta', { text: 'Je cher' }))
		})

		await act(async () => {
			result.current.abort()
			await pending
		})

		expect(signal?.aborted).toBe(true)
		expect(result.current.state.status).toBe('idle')
		expect(result.current.state.error).toBeUndefined()
	})

	it('aborts the request when the page goes away', async () => {
		let signal: AbortSignal | undefined
		fetchMock.mockImplementation((_url: string, init: RequestInit) => {
			signal = init.signal as AbortSignal
			return new Promise(() => undefined)
		})
		const { result, unmount } = setup()
		act(() => {
			void result.current.send('Salut')
		})
		await waitFor(() => expect(signal).toBeDefined())

		unmount()

		expect(signal?.aborted).toBe(true)
	})

	it.each([
		[409, 'turn_in_progress', /répond déjà/],
		[409, 'conversation_full', /limite/],
		[409, 'captain_not_configured', /Paramètres › Modèles/],
		[403, 'forbidden', /agents ne peuvent pas/],
	])('reports a %s %s answer as a message', async (status, code, expected) => {
		fetchMock.mockResolvedValue(json(status, { code, status, message: 'x' }))
		const { result } = setup()

		await act(async () => {
			await result.current.send('Salut')
		})

		expect(result.current.state.status).toBe('idle')
		expect(result.current.state.error).toMatch(expected)
	})

	it('reports an error event of the stream and keeps the streamed text out of the draft once stored', async () => {
		fetchMock.mockResolvedValue(
			sseResponse([
				sseEvent('delta', { text: 'Début' }),
				sseEvent('error', {
					code: 'provider_error',
					message: 'quota exceeded',
				}),
			]),
		)
		const { result } = setup()

		await act(async () => {
			await result.current.send('Salut')
		})

		expect(result.current.state.error).toBe(
			'Le modèle n’a pas pu répondre : quota exceeded',
		)
		expect(result.current.state.draft.text).toBe('')
		act(() => result.current.dismissError())
		expect(result.current.state.error).toBeUndefined()
	})

	it('reports a network failure', async () => {
		fetchMock.mockRejectedValue(new TypeError('Failed to fetch'))
		const { result } = setup()

		await act(async () => {
			await result.current.send('Salut')
		})

		expect(result.current.state.error).toBe(
			'Impossible de joindre l’API pour parler au Capitaine.',
		)
	})
})
