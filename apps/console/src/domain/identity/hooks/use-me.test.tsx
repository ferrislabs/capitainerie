import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useMe } from '#/domain/identity/hooks/use-me'
import { clearAuth, setAuth } from '#/store/auth.store'

const ME = {
	subject: 'sub-1',
	username: 'ada',
	email: 'ada@example.com',
	display_name: 'Ada Lovelace',
	organization: { id: 'org-1', slug: 'acme', name: 'Acme' },
}

function wrapper({ children }: { children: ReactNode }) {
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	})
	return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

describe('useMe', () => {
	const fetchMock = vi.fn()

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		fetchMock.mockResolvedValue(
			new Response(JSON.stringify(ME), {
				status: 200,
				headers: { 'Content-Type': 'application/json' },
			}),
		)
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('does not fetch without an access token', () => {
		renderHook(() => useMe(), { wrapper })

		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('sends the bearer token and returns the parsed body', async () => {
		setAuth({ accessToken: 'tok', isAuthenticated: true })

		const { result } = renderHook(() => useMe(), { wrapper })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		expect(result.current.data).toEqual(ME)
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/me')
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
	})

	it('fetches once the token appears', async () => {
		const { result } = renderHook(() => useMe(), { wrapper })

		act(() => setAuth({ accessToken: 'late', isAuthenticated: true }))

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
	})
})
