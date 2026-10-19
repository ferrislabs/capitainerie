import { useCallback } from 'react'
import type { Schemas } from '#/api/api.client'
import {
	postFile,
	type UploadFailure,
	useCancellableUpload,
} from '#/domain/runs/hooks/use-cancellable-upload'
import { transcriptionErrorMessage } from '#/domain/runs/transcription-errors'
import { isApiRequestError } from '#/lib/api-error'

export const TRANSCRIPTIONS_PATH = '/api/v1/transcriptions'

export type TranscribeOutcome =
	| { kind: 'done'; text: string; model: string }
	| { kind: 'cancelled' }
	| { kind: 'failed'; message: string; notConfigured: boolean }

export function postAudio(
	file: File,
	signal: AbortSignal,
): Promise<Schemas.TranscriptionResponse> {
	return postFile<Schemas.TranscriptionResponse>(
		TRANSCRIPTIONS_PATH,
		file,
		signal,
	)
}

function describeFailure(error: unknown): UploadFailure {
	return {
		message: transcriptionErrorMessage(error),
		notConfigured:
			isApiRequestError(error) && error.code === 'transcriber_not_configured',
	}
}

export function useTranscribe() {
	const upload = useCancellableUpload(postAudio, describeFailure)
	const uploadFile = upload.run

	const run = useCallback(
		async (file: File): Promise<TranscribeOutcome> => {
			const outcome = await uploadFile(file)
			return outcome.kind === 'done'
				? { kind: 'done', text: outcome.value.text, model: outcome.value.model }
				: outcome
		},
		[uploadFile],
	)

	return {
		fileName: upload.fileName,
		isPending: upload.isPending,
		run,
		cancel: upload.cancel,
	}
}
