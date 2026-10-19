import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import {
	dictationFile,
	useVoiceInput,
} from '#/domain/chat/hooks/use-voice-input'
import { json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

class FakeRecorder {
	static last: FakeRecorder | undefined
	mimeType = 'audio/webm;codecs=opus'
	ondataavailable: ((event: { data: Blob }) => void) | null = null
	onstop: (() => void) | null = null
	started = false

	constructor(readonly stream: MediaStream) {
		FakeRecorder.last = this
	}

	start() {
		this.started = true
	}

	stop() {
		this.ondataavailable?.({
			data: new Blob(['audio'], { type: this.mimeType }),
		})
		this.onstop?.()
	}
}

describe('dictationFile', () => {
	it('names the file after the recorded format, which the API accepts', () => {
		const chunks = [new Blob(['a'])]

		expect(dictationFile(chunks, 'audio/webm;codecs=opus').name).toBe(
			'dictee.webm',
		)
		expect(dictationFile(chunks, 'audio/mp4').name).toBe('dictee.m4a')
		expect(dictationFile(chunks, 'audio/ogg;codecs=opus').name).toBe(
			'dictee.ogg',
		)
		expect(dictationFile(chunks, '').name).toBe('dictee.webm')
		expect(dictationFile(chunks, 'audio/mp4').type).toBe('audio/mp4')
	})
})

describe('useVoiceInput', () => {
	const fetchMock = vi.fn()
	const stopTrack = vi.fn()
	const getUserMedia = vi.fn()

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		vi.stubGlobal('MediaRecorder', FakeRecorder)
		vi.stubGlobal('navigator', { mediaDevices: { getUserMedia } })
		getUserMedia.mockResolvedValue({ getTracks: () => [{ stop: stopTrack }] })
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		FakeRecorder.last = undefined
	})

	afterEach(() => {
		fetchMock.mockReset()
		stopTrack.mockReset()
		getUserMedia.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('records, sends the audio to the existing transcription and hands the text over', async () => {
		fetchMock.mockResolvedValue(
			json(200, { text: ' Ajoute un README ', model: 'voxtral-mini-latest' }),
		)
		const onText = vi.fn()
		const { result } = renderHook(() => useVoiceInput(onText))
		expect(result.current.supported).toBe(true)

		act(() => result.current.toggle())
		await waitFor(() => expect(result.current.state).toBe('recording'))
		expect(FakeRecorder.last?.started).toBe(true)

		act(() => result.current.toggle())

		await waitFor(() => expect(onText).toHaveBeenCalledWith('Ajoute un README'))
		const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
		expect(url).toBe('http://api.test/api/v1/transcriptions')
		const file = (init.body as FormData).get('file') as File
		expect(file.name).toBe('dictee.webm')
		expect(stopTrack).toHaveBeenCalled()
		expect(result.current.state).toBe('idle')
		expect(result.current.error).toBeUndefined()
	})

	it('reports a missing transcriber without losing the field', async () => {
		fetchMock.mockResolvedValue(
			json(409, {
				code: 'transcriber_not_configured',
				status: 409,
				message: 'x',
			}),
		)
		const onText = vi.fn()
		const { result } = renderHook(() => useVoiceInput(onText))

		act(() => result.current.toggle())
		await waitFor(() => expect(result.current.state).toBe('recording'))
		act(() => result.current.toggle())

		await waitFor(() =>
			expect(result.current.error).toMatch(/Aucun transcripteur/),
		)
		expect(onText).not.toHaveBeenCalled()
	})

	it('drops the recording on cancel and sends nothing', async () => {
		const onText = vi.fn()
		const { result } = renderHook(() => useVoiceInput(onText))
		act(() => result.current.toggle())
		await waitFor(() => expect(result.current.state).toBe('recording'))

		act(() => result.current.cancel())

		expect(result.current.state).toBe('idle')
		expect(fetchMock).not.toHaveBeenCalled()
		expect(stopTrack).toHaveBeenCalled()
	})

	it('explains a refused microphone', async () => {
		getUserMedia.mockRejectedValue(new DOMException('no', 'NotAllowedError'))
		const { result } = renderHook(() => useVoiceInput(vi.fn()))

		act(() => result.current.toggle())

		await waitFor(() => expect(result.current.error).toMatch(/micro/))
		expect(result.current.state).toBe('idle')
	})

	it('is not supported without a recorder', () => {
		vi.stubGlobal('MediaRecorder', undefined)
		const { result } = renderHook(() => useVoiceInput(vi.fn()))

		expect(result.current.supported).toBe(false)
	})
})
