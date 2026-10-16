import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fetcher } from '#/api/api.fetch'
import { ApiRequestError, shouldRetry } from '#/lib/api-error'
import { authStore, clearAuth, setAuth } from '#/store/auth.store'

function jsonResponse(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('fetcher', () => {
	const fetchMock = vi.fn()

	beforeEach(() => {
		vi.stubGlobal('fetch', fetchMock)
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('injects the bearer token from the auth store', async () => {
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		fetchMock.mockResolvedValue(jsonResponse(200, {}))

		await fetcher({ method: 'get', url: new URL('http://api.test/runs') })

		const init = fetchMock.mock.calls[0][1] as RequestInit
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
		expect(authStore.state.accessToken).toBe('tok')
	})

	it('omits Authorization when signed out', async () => {
		fetchMock.mockResolvedValue(jsonResponse(200, {}))

		await fetcher({ method: 'get', url: new URL('http://api.test/runs') })

		const init = fetchMock.mock.calls[0][1] as RequestInit
		expect(new Headers(init.headers).has('Authorization')).toBe(false)
	})

	it('throws a typed ApiRequestError on non-2xx', async () => {
		fetchMock.mockResolvedValue(
			jsonResponse(404, { code: 'run_not_found', message: 'Run introuvable' }),
		)

		const error = await fetcher({
			method: 'get',
			url: new URL('http://api.test/runs/1'),
		}).catch((e: unknown) => e)

		expect(error).toBeInstanceOf(ApiRequestError)
		expect(error).toMatchObject({
			status: 404,
			code: 'run_not_found',
			message: 'Run introuvable',
		})
	})

	it('falls back to a generated code when the body is not JSON', async () => {
		fetchMock.mockResolvedValue(new Response('boom', { status: 502 }))

		const error = await fetcher({
			method: 'get',
			url: new URL('http://api.test/x'),
		}).catch((e: unknown) => e)

		expect(error).toMatchObject({ status: 502, code: 'http_502' })
	})
})

describe('shouldRetry', () => {
	const err = (status: number) => new ApiRequestError(status, 'c', 'm')

	it('retries only 408, 429 and 5xx', () => {
		expect(shouldRetry(0, err(408))).toBe(true)
		expect(shouldRetry(0, err(429))).toBe(true)
		expect(shouldRetry(0, err(503))).toBe(true)
		expect(shouldRetry(0, err(400))).toBe(false)
		expect(shouldRetry(0, err(404))).toBe(false)
		expect(shouldRetry(0, new Error('network'))).toBe(false)
	})

	it('stops after three retries', () => {
		expect(shouldRetry(2, err(500))).toBe(true)
		expect(shouldRetry(3, err(500))).toBe(false)
	})
})
