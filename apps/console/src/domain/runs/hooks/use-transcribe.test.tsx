import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useTranscribe } from '#/domain/runs/hooks/use-transcribe'
import { clearAuth, setAuth } from '#/store/auth.store'

function reply(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('useTranscribe', () => {
	const fetchMock = vi.fn()

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('posts the file as multipart with the bearer token and returns the text', async () => {
		fetchMock.mockResolvedValue(
			reply(200, { text: 'Bonjour', model: 'voxtral-mini-latest' }),
		)
		const { result } = renderHook(() => useTranscribe())
		const file = new File(['abc'], 'réunion.m4a', { type: 'audio/mp4' })

		let outcome: Awaited<ReturnType<typeof result.current.run>> | undefined
		await act(async () => {
			outcome = await result.current.run(file)
		})

		expect(outcome).toEqual({
			kind: 'done',
			text: 'Bonjour',
			model: 'voxtral-mini-latest',
		})
		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
		expect(url).toBe('http://api.test/api/v1/transcriptions')
		expect(init.method).toBe('POST')
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
		expect(new Headers(init.headers).has('Content-Type')).toBe(false)
		const form = init.body as FormData
		expect(form).toBeInstanceOf(FormData)
		expect([...form.keys()]).toEqual(['file'])
		expect((form.get('file') as File).name).toBe('réunion.m4a')
		expect(result.current.isPending).toBe(false)
	})

	it('exposes the file name while pending and aborts the request on cancel', async () => {
		let signal: AbortSignal | undefined
		fetchMock.mockImplementation(
			(_url: string, init: RequestInit) =>
				new Promise((_resolve, reject) => {
					signal = init.signal as AbortSignal
					signal.addEventListener('abort', () =>
						reject(new DOMException('Aborted', 'AbortError')),
					)
				}),
		)
		const { result } = renderHook(() => useTranscribe())

		let pending: ReturnType<typeof result.current.run> | undefined
		act(() => {
			pending = result.current.run(new File(['x'], 'long.mp3'))
		})
		await waitFor(() => expect(result.current.fileName).toBe('long.mp3'))
		expect(result.current.isPending).toBe(true)
		expect(signal?.aborted).toBe(false)

		act(() => result.current.cancel())

		expect(signal?.aborted).toBe(true)
		await act(async () => {
			expect(await pending).toEqual({ kind: 'cancelled' })
		})
		expect(result.current.isPending).toBe(false)
		expect(result.current.fileName).toBeUndefined()
	})

	it('aborts the request when the component goes away', () => {
		let signal: AbortSignal | undefined
		fetchMock.mockImplementation((_url: string, init: RequestInit) => {
			signal = init.signal as AbortSignal
			return new Promise(() => undefined)
		})
		const { result, unmount } = renderHook(() => useTranscribe())
		act(() => {
			void result.current.run(new File(['x'], 'a.wav'))
		})

		unmount()

		expect(signal?.aborted).toBe(true)
	})

	it('reports a missing transcriber, a refusal and a network failure', async () => {
		const { result } = renderHook(() => useTranscribe())
		const run = async () => {
			let outcome: Awaited<ReturnType<typeof result.current.run>> | undefined
			await act(async () => {
				outcome = await result.current.run(new File(['x'], 'a.ogg'))
			})
			return outcome
		}

		fetchMock.mockResolvedValueOnce(
			reply(409, {
				code: 'transcriber_not_configured',
				status: 409,
				message: 'none',
			}),
		)
		expect(await run()).toMatchObject({ kind: 'failed', notConfigured: true })

		fetchMock.mockResolvedValueOnce(
			reply(502, { code: 'bad_gateway', status: 502, message: 'boom' }),
		)
		expect(await run()).toEqual({
			kind: 'failed',
			notConfigured: false,
			message: 'Le fournisseur n’a pas pu transcrire ce fichier : boom',
		})

		fetchMock.mockRejectedValueOnce(new TypeError('Failed to fetch'))
		expect(await run()).toMatchObject({ kind: 'failed', notConfigured: false })
	})
})
