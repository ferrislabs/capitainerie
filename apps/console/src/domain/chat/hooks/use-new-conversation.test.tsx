import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import {
	appendDictation,
	useNewConversation,
} from '#/domain/chat/hooks/use-new-conversation'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const navigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
	useNavigate: () => navigate,
}))

describe('appendDictation', () => {
	it('starts an empty field and appends to a filled one', () => {
		expect(appendDictation('', 'Bonjour')).toBe('Bonjour')
		expect(appendDictation('  ', 'Bonjour')).toBe('Bonjour')
		expect(appendDictation('Ajoute un README ', 'au dépôt test')).toBe(
			'Ajoute un README au dépôt test',
		)
	})
})

describe('useNewConversation', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		navigate.mockReset()
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			if (url.pathname === '/api/v1/model-assignments') {
				return Promise.resolve(
					json(200, {
						assignments: {
							extractor: { provider: 'mistral', model: 'm' },
							judge: { provider: 'mistral', model: 'm' },
							writer: { provider: 'mistral', model: 'mistral-small' },
							embeddings: { provider: 'mistral', model: 'e' },
							captain: { provider: 'anthropic', model: 'claude-sonnet-5-5' },
						},
					}),
				)
			}
			if (init.method === 'POST' && url.pathname === '/api/v1/conversations') {
				return Promise.resolve(json(201, { id: 'c9' }))
			}
			return Promise.resolve(
				json(404, { code: 'not_found', status: 404, message: '' }),
			)
		})
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('creates the conversation with the trimmed message, then opens it', async () => {
		const { result } = renderHook(() => useNewConversation(), {
			wrapper: harness.wrapper,
		})
		act(() => result.current.setValue('  Ajoute un README  '))

		act(() => result.current.submit())

		await waitFor(() =>
			expect(navigate).toHaveBeenCalledWith({
				to: '/chat/$id',
				params: { id: 'c9' },
			}),
		)
		const post = fetchMock.mock.calls.find(
			([, init]) => init?.method === 'POST',
		)
		expect(JSON.parse(String(post?.[1].body))).toEqual({
			message: 'Ajoute un README',
		})
	})

	it('does nothing for an empty message', () => {
		const { result } = renderHook(() => useNewConversation(), {
			wrapper: harness.wrapper,
		})
		act(() => result.current.setValue('   '))

		act(() => result.current.submit())

		expect(
			fetchMock.mock.calls.some(([, init]) => init?.method === 'POST'),
		).toBe(false)
	})

	it('shows the failure of the creation in French', async () => {
		fetchMock.mockImplementation((_url: URL, init: RequestInit) =>
			Promise.resolve(
				init.method === 'POST'
					? json(403, { code: 'forbidden', status: 403, message: 'x' })
					: json(200, { assignments: null }),
			),
		)
		const { result } = renderHook(() => useNewConversation(), {
			wrapper: harness.wrapper,
		})
		act(() => result.current.setValue('Salut'))

		act(() => result.current.submit())

		await waitFor(() =>
			expect(result.current.errorMessage).toBe(
				'Les agents ne peuvent pas utiliser le chat.',
			),
		)
		expect(navigate).not.toHaveBeenCalled()
	})

	it('shows the Captain model, then the writer’s without one', async () => {
		const { result } = renderHook(() => useNewConversation(), {
			wrapper: harness.wrapper,
		})

		await waitFor(() =>
			expect(result.current.captain).toEqual({
				kind: 'ready',
				model: 'claude-sonnet-5-5',
			}),
		)
	})
})
