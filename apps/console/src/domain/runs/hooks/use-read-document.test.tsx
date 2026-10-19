import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useReadDocument } from '#/domain/runs/hooks/use-read-document'
import { clearAuth, setAuth } from '#/store/auth.store'

function reply(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

describe('useReadDocument', () => {
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

	it('posts the pdf as multipart with the bearer token and returns the text', async () => {
		fetchMock.mockResolvedValue(
			reply(200, { text: '# Titre', model: 'mistral-ocr-latest', pages: 3 }),
		)
		const { result } = renderHook(() => useReadDocument())
		const file = new File(['%PDF-1'], 'rapport.pdf', {
			type: 'application/pdf',
		})

		let outcome: Awaited<ReturnType<typeof result.current.run>> | undefined
		await act(async () => {
			outcome = await result.current.run(file)
		})

		expect(outcome).toEqual({
			kind: 'done',
			text: '# Titre',
			model: 'mistral-ocr-latest',
			pages: 3,
		})
		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
		expect(url).toBe('http://api.test/api/v1/documents/text')
		expect(init.method).toBe('POST')
		expect(new Headers(init.headers).get('Authorization')).toBe('Bearer tok')
		expect(new Headers(init.headers).has('Content-Type')).toBe(false)
		const form = init.body as FormData
		expect([...form.keys()]).toEqual(['file'])
		expect((form.get('file') as File).name).toBe('rapport.pdf')
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
		const { result } = renderHook(() => useReadDocument())

		let pending: ReturnType<typeof result.current.run> | undefined
		act(() => {
			pending = result.current.run(new File(['x'], 'long.pdf'))
		})
		await waitFor(() => expect(result.current.fileName).toBe('long.pdf'))
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
		const { result, unmount } = renderHook(() => useReadDocument())
		act(() => {
			void result.current.run(new File(['x'], 'a.pdf'))
		})

		unmount()

		expect(signal?.aborted).toBe(true)
	})

	it('aborts the previous request when a new file is read', () => {
		const signals: Array<AbortSignal> = []
		fetchMock.mockImplementation((_url: string, init: RequestInit) => {
			signals.push(init.signal as AbortSignal)
			return new Promise(() => undefined)
		})
		const { result } = renderHook(() => useReadDocument())

		act(() => {
			void result.current.run(new File(['x'], 'a.pdf'))
		})
		act(() => {
			void result.current.run(new File(['y'], 'b.pdf'))
		})

		expect(signals[0].aborted).toBe(true)
		expect(signals[1].aborted).toBe(false)
	})

	it('reports a missing reader, a refusal and a network failure', async () => {
		const { result } = renderHook(() => useReadDocument())
		const run = async () => {
			let outcome: Awaited<ReturnType<typeof result.current.run>> | undefined
			await act(async () => {
				outcome = await result.current.run(new File(['x'], 'a.pdf'))
			})
			return outcome
		}

		fetchMock.mockResolvedValueOnce(
			reply(409, { code: 'reader_not_configured', status: 409, message: 'x' }),
		)
		expect(await run()).toMatchObject({ kind: 'failed', notConfigured: true })

		fetchMock.mockResolvedValueOnce(
			reply(502, {
				code: 'bad_gateway',
				status: 502,
				message: 'Le service de lecture des PDF est injoignable.',
			}),
		)
		expect(await run()).toEqual({
			kind: 'failed',
			notConfigured: false,
			message: 'Le service de lecture des PDF est injoignable.',
		})

		fetchMock.mockRejectedValueOnce(new TypeError('Failed to fetch'))
		expect(await run()).toMatchObject({ kind: 'failed', notConfigured: false })
	})
})
