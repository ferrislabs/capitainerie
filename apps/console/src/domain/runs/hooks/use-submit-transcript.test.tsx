import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { RUNS_QUERY_KEY } from '#/domain/runs/hooks/use-runs'
import { useSubmitTranscript } from '#/domain/runs/hooks/use-submit-transcript'
import { clearAuth, setAuth } from '#/store/auth.store'

describe('useSubmitTranscript', () => {
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

	it('posts the transcript and invalidates the runs list', async () => {
		fetchMock.mockResolvedValue(
			new Response(JSON.stringify({ transcript_id: 't', run_id: 'r' }), {
				status: 202,
				headers: { 'Content-Type': 'application/json' },
			}),
		)
		client.setQueryData([{ _id: '/api/v1/runs', path: { limit: 20 } }], {
			items: [],
			total: 0,
		})
		const { result } = renderHook(() => useSubmitTranscript(), { wrapper })

		result.current.mutate({ body: { title: 'Réunion', body: 'Bonjour' } })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/transcripts')
		expect(init.method).toBe('POST')
		expect(JSON.parse(String(init.body))).toEqual({
			title: 'Réunion',
			body: 'Bonjour',
		})
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
		expect(
			client.getQueryState(
				client.getQueryCache().findAll({ queryKey: RUNS_QUERY_KEY })[0]
					.queryKey,
			)?.isInvalidated,
		).toBe(true)
	})

	it('exposes the server message on 422', async () => {
		fetchMock.mockResolvedValue(
			new Response(
				JSON.stringify({ code: 'invalid', status: 422, message: 'Titre vide' }),
				{ status: 422, headers: { 'Content-Type': 'application/json' } },
			),
		)
		const { result } = renderHook(() => useSubmitTranscript(), { wrapper })

		result.current.mutate({ body: { title: ' ', body: 'x' } })

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.error?.message).toBe('Titre vide')
	})
})
