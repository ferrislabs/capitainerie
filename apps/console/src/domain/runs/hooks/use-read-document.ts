import { useCallback } from 'react'
import type { Schemas } from '#/api/api.client'
import { documentErrorMessage } from '#/domain/runs/document-errors'
import {
	postFile,
	type UploadFailure,
	useCancellableUpload,
} from '#/domain/runs/hooks/use-cancellable-upload'
import { isApiRequestError } from '#/lib/api-error'

export const DOCUMENT_TEXT_PATH = '/api/v1/documents/text'

export type ReadDocumentOutcome =
	| { kind: 'done'; text: string; model: string; pages: number }
	| { kind: 'cancelled' }
	| { kind: 'failed'; message: string; notConfigured: boolean }

export function postPdf(
	file: File,
	signal: AbortSignal,
): Promise<Schemas.DocumentTextResponse> {
	return postFile<Schemas.DocumentTextResponse>(
		DOCUMENT_TEXT_PATH,
		file,
		signal,
	)
}

function describeFailure(error: unknown): UploadFailure {
	return {
		message: documentErrorMessage(error),
		notConfigured:
			isApiRequestError(error) && error.code === 'reader_not_configured',
	}
}

export function useReadDocument() {
	const upload = useCancellableUpload(postPdf, describeFailure)
	const uploadFile = upload.run

	const run = useCallback(
		async (file: File): Promise<ReadDocumentOutcome> => {
			const outcome = await uploadFile(file)
			return outcome.kind === 'done'
				? {
						kind: 'done',
						text: outcome.value.text,
						model: outcome.value.model,
						pages: outcome.value.pages,
					}
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
