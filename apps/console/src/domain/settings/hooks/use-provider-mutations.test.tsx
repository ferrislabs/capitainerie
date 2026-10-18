import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import {
	oneShot,
	useDeleteProvider,
	useSaveProvider,
	useTestProvider,
} from '#/domain/settings/hooks/use-provider-mutations'
import { ADMIN_ONLY_PROVIDERS } from '#/domain/settings/providers'
import { ANTHROPIC_PROVIDER, QWEN } from '#/domain/settings/test-fixtures'
import { clearAuth, setAuth } from '#/store/auth.store'

const SECRET = 'sk-ant-secret-a1b2'

describe('provider mutations', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		harness.seed('/api/v1/providers', { providers: [] })
		harness.seed('/api/v1/models', { models: [], errors: [] })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	function lastCall() {
		const [url, init] = fetchMock.mock.calls.at(-1) as [URL, RequestInit]
		return { url: String(url), init }
	}

	describe('useSaveProvider', () => {
		it('puts the key, invalidates providers and models, and keeps no key', async () => {
			fetchMock.mockResolvedValue(json(200, ANTHROPIC_PROVIDER))
			const onSuccess = vi.fn()
			const { result } = renderHook(() => useSaveProvider(), {
				wrapper: harness.wrapper,
			})

			act(() =>
				result.current.save(
					'anthropic',
					{ apiKey: SECRET, baseUrl: 'http://ignored' },
					{ onSuccess },
				),
			)

			await waitFor(() => expect(result.current.savedKind).toBe('anthropic'))
			const { url, init } = lastCall()
			expect(url).toBe('http://api.test/api/v1/providers/anthropic')
			expect(init.method).toBe('PUT')
			expect(JSON.parse(String(init.body))).toEqual({ api_key: SECRET })
			expect(onSuccess).toHaveBeenCalledOnce()
			expect(harness.isInvalidated('/api/v1/providers')).toBe(true)
			expect(harness.isInvalidated('/api/v1/models')).toBe(true)

			expect(JSON.stringify(result.current.variables)).not.toContain(SECRET)
			expect(result.current.variables?.takeKey()).toBe('')
			const cached = harness.client.getMutationCache().getAll()
			expect(JSON.stringify(cached.map((m) => m.state))).not.toContain(SECRET)
		})

		it('sends the base URL for Compatible OpenAI, with an empty key allowed', async () => {
			fetchMock.mockResolvedValue(json(200, ANTHROPIC_PROVIDER))
			const { result } = renderHook(() => useSaveProvider(), {
				wrapper: harness.wrapper,
			})

			act(() =>
				result.current.save('openai_compatible', {
					apiKey: '',
					baseUrl: ' http://localhost:4120/v1 ',
				}),
			)

			await waitFor(() => expect(fetchMock).toHaveBeenCalled())
			expect(JSON.parse(String(lastCall().init.body))).toEqual({
				api_key: '',
				base_url: 'http://localhost:4120/v1',
			})
		})

		it('surfaces the server message on 422 without invalidating', async () => {
			fetchMock.mockResolvedValue(
				json(422, { code: 'invalid', status: 422, message: 'invalid api key' }),
			)
			const { result } = renderHook(() => useSaveProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.save('mistral', { apiKey: SECRET, baseUrl: '' }))

			await waitFor(() =>
				expect(result.current.errorMessage).toBe('invalid api key'),
			)
			expect(harness.isInvalidated('/api/v1/providers')).toBe(false)
			expect(JSON.stringify(result.current.variables)).not.toContain(SECRET)
		})

		it('explains a 403 as admin-only', async () => {
			fetchMock.mockResolvedValue(
				json(403, { code: 'forbidden', status: 403, message: 'forbidden' }),
			)
			const { result } = renderHook(() => useSaveProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.save('mistral', { apiKey: SECRET, baseUrl: '' }))

			await waitFor(() =>
				expect(result.current.errorMessage).toBe(ADMIN_ONLY_PROVIDERS),
			)
		})
	})

	it('oneShot hands the value out once', () => {
		const take = oneShot('k')
		expect(take()).toBe('k')
		expect(take()).toBe('')
	})

	describe('useDeleteProvider', () => {
		it('deletes and invalidates providers and models', async () => {
			fetchMock.mockResolvedValue(new Response(null, { status: 204 }))
			const onSuccess = vi.fn()
			const { result } = renderHook(() => useDeleteProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.remove('mistral', { onSuccess }))

			await waitFor(() => expect(onSuccess).toHaveBeenCalledOnce())
			const { url, init } = lastCall()
			expect(url).toBe('http://api.test/api/v1/providers/mistral')
			expect(init.method).toBe('DELETE')
			expect(harness.isInvalidated('/api/v1/providers')).toBe(true)
			expect(harness.isInvalidated('/api/v1/models')).toBe(true)
		})

		it('explains a 403 as admin-only', async () => {
			fetchMock.mockResolvedValue(
				json(403, { code: 'forbidden', status: 403, message: 'forbidden' }),
			)
			const { result } = renderHook(() => useDeleteProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.remove('mistral'))

			await waitFor(() =>
				expect(result.current.errorMessage).toBe(ADMIN_ONLY_PROVIDERS),
			)
		})
	})

	describe('useTestProvider', () => {
		it('posts the test and records the model count', async () => {
			fetchMock.mockResolvedValue(json(200, { ok: true, models: [QWEN, QWEN] }))
			const { result } = renderHook(() => useTestProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.test('openai_compatible'))

			await waitFor(() =>
				expect(result.current.results.openai_compatible).toEqual({
					ok: true,
					modelCount: 2,
				}),
			)
			const { url, init } = lastCall()
			expect(url).toBe(
				'http://api.test/api/v1/providers/openai_compatible/test',
			)
			expect(init.method).toBe('POST')
			expect(harness.isInvalidated('/api/v1/models')).toBe(true)
		})

		it('records the provider error, then forgets it', async () => {
			fetchMock.mockResolvedValue(
				json(200, { ok: false, error: 'invalid x-api-key', models: [] }),
			)
			const { result } = renderHook(() => useTestProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.test('anthropic'))

			await waitFor(() =>
				expect(result.current.results.anthropic).toEqual({
					ok: false,
					error: 'invalid x-api-key',
				}),
			)
			act(() => result.current.forget('anthropic'))
			expect(result.current.results.anthropic).toBeUndefined()
		})

		it('records a 403 as admin-only', async () => {
			fetchMock.mockResolvedValue(
				json(403, { code: 'forbidden', status: 403, message: 'forbidden' }),
			)
			const { result } = renderHook(() => useTestProvider(), {
				wrapper: harness.wrapper,
			})

			act(() => result.current.test('anthropic'))

			await waitFor(() =>
				expect(result.current.results.anthropic).toEqual({
					ok: false,
					error: ADMIN_ONLY_PROVIDERS,
				}),
			)
		})
	})
})
