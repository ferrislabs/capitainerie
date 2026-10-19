import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranscribe } from '#/domain/runs/hooks/use-transcribe'

export type VoiceState = 'idle' | 'recording' | 'transcribing'

const MIME_EXTENSIONS: ReadonlyArray<readonly [string, string]> = [
	['mp4', 'm4a'],
	['ogg', 'ogg'],
	['wav', 'wav'],
]

export function dictationFile(chunks: Array<Blob>, mimeType: string): File {
	const type = mimeType || 'audio/webm'
	const extension =
		MIME_EXTENSIONS.find(([marker]) => type.includes(marker))?.[1] ?? 'webm'
	return new File(chunks, `dictee.${extension}`, { type })
}

export function voiceSupported(): boolean {
	return (
		typeof MediaRecorder !== 'undefined' &&
		typeof navigator !== 'undefined' &&
		typeof navigator.mediaDevices?.getUserMedia === 'function'
	)
}

export function useVoiceInput(onText: (text: string) => void) {
	const transcription = useTranscribe()
	const run = transcription.run
	const [recording, setRecording] = useState(false)
	const [error, setError] = useState<string>()
	const recorder = useRef<MediaRecorder | null>(null)
	const discard = useRef(false)

	useEffect(
		() => () => {
			discard.current = true
			recorder.current?.stop()
		},
		[],
	)

	const finish = useCallback(
		async (file: File) => {
			const outcome = await run(file)
			if (outcome.kind === 'done') {
				const text = outcome.text.trim()
				if (text !== '') onText(text)
			}
			if (outcome.kind === 'failed') setError(outcome.message)
		},
		[run, onText],
	)

	const start = useCallback(async () => {
		setError(undefined)
		let stream: MediaStream
		try {
			stream = await navigator.mediaDevices.getUserMedia({ audio: true })
		} catch {
			setError(
				'Le micro n’est pas accessible : autorisez-le dans le navigateur.',
			)
			return
		}
		const chunks: Array<Blob> = []
		const own = new MediaRecorder(stream)
		discard.current = false
		own.ondataavailable = (event) => {
			if (event.data.size > 0) chunks.push(event.data)
		}
		own.onstop = () => {
			for (const track of stream.getTracks()) track.stop()
			recorder.current = null
			setRecording(false)
			if (discard.current || chunks.length === 0) return
			void finish(dictationFile(chunks, own.mimeType))
		}
		recorder.current = own
		own.start()
		setRecording(true)
	}, [finish])

	const toggle = useCallback(() => {
		if (recorder.current) {
			recorder.current.stop()
			return
		}
		if (!transcription.isPending) void start()
	}, [start, transcription.isPending])

	const cancel = useCallback(() => {
		if (recorder.current) {
			discard.current = true
			recorder.current.stop()
			return
		}
		transcription.cancel()
	}, [transcription])

	const state: VoiceState = recording
		? 'recording'
		: transcription.isPending
			? 'transcribing'
			: 'idle'

	return { state, error, toggle, cancel, supported: voiceSupported() }
}
