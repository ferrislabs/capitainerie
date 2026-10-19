import { useCallback, useEffect, useRef, useState } from 'react'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { authHeaders, toApiError } from '#/api/api.fetch'
import { transcriptionErrorMessage } from '#/domain/runs/transcription-errors'
import { isApiRequestError } from '#/lib/api-error'

export const TRANSCRIPTIONS_PATH = '/api/v1/transcriptions'

export type TranscribeOutcome =
	| { kind: 'done'; text: string; model: string }
	| { kind: 'cancelled' }
	| { kind: 'failed'; message: string; notConfigured: boolean }

export async function postAudio(
	file: File,
	signal: AbortSignal,
): Promise<Schemas.TranscriptionResponse> {
	const body = new FormData()
	body.append('file', file, file.name)
	const response = await fetch(`${api.baseUrl}${TRANSCRIPTIONS_PATH}`, {
		method: 'POST',
		headers: authHeaders(),
		body,
		signal,
	})
	if (!response.ok) throw await toApiError(response)
	return (await response.json()) as Schemas.TranscriptionResponse
}

export function useTranscribe() {
	const controller = useRef<AbortController | null>(null)
	const [fileName, setFileName] = useState<string>()

	useEffect(() => () => controller.current?.abort(), [])

	const run = useCallback(async (file: File): Promise<TranscribeOutcome> => {
		controller.current?.abort()
		const own = new AbortController()
		controller.current = own
		setFileName(file.name)
		try {
			const result = await postAudio(file, own.signal)
			return { kind: 'done', text: result.text, model: result.model }
		} catch (error) {
			if (own.signal.aborted) return { kind: 'cancelled' }
			return {
				kind: 'failed',
				message: transcriptionErrorMessage(error),
				notConfigured:
					isApiRequestError(error) &&
					error.code === 'transcriber_not_configured',
			}
		} finally {
			if (controller.current === own) {
				controller.current = null
				setFileName(undefined)
			}
		}
	}, [])

	const cancel = useCallback(() => controller.current?.abort(), [])

	return { fileName, isPending: fileName !== undefined, run, cancel }
}
