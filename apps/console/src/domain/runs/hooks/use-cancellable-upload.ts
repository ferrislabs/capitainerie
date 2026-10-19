import { useCallback, useEffect, useRef, useState } from 'react'
import { api } from '#/api'
import { authHeaders, toApiError } from '#/api/api.fetch'

export interface UploadFailure {
	message: string
	notConfigured: boolean
}

export type UploadOutcome<T> =
	| { kind: 'done'; value: T }
	| { kind: 'cancelled' }
	| ({ kind: 'failed' } & UploadFailure)

export async function postFile<T>(
	path: string,
	file: File,
	signal: AbortSignal,
): Promise<T> {
	const body = new FormData()
	body.append('file', file, file.name)
	const response = await fetch(`${api.baseUrl}${path}`, {
		method: 'POST',
		headers: authHeaders(),
		body,
		signal,
	})
	if (!response.ok) throw await toApiError(response)
	return (await response.json()) as T
}

export function useCancellableUpload<T>(
	post: (file: File, signal: AbortSignal) => Promise<T>,
	describeFailure: (error: unknown) => UploadFailure,
) {
	const controller = useRef<AbortController | null>(null)
	const [fileName, setFileName] = useState<string>()

	useEffect(() => () => controller.current?.abort(), [])

	const run = useCallback(
		async (file: File): Promise<UploadOutcome<T>> => {
			controller.current?.abort()
			const own = new AbortController()
			controller.current = own
			setFileName(file.name)
			try {
				return { kind: 'done', value: await post(file, own.signal) }
			} catch (error) {
				if (own.signal.aborted) return { kind: 'cancelled' }
				return { kind: 'failed', ...describeFailure(error) }
			} finally {
				if (controller.current === own) {
					controller.current = null
					setFileName(undefined)
				}
			}
		},
		[post, describeFailure],
	)

	const cancel = useCallback(() => controller.current?.abort(), [])

	return { fileName, isPending: fileName !== undefined, run, cancel }
}
