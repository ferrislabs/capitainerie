export class ApiRequestError extends Error {
	readonly status: number
	readonly code: string

	constructor(status: number, code: string, message: string) {
		super(message)
		this.name = 'ApiRequestError'
		this.status = status
		this.code = code
	}
}

export function isApiRequestError(error: unknown): error is ApiRequestError {
	return error instanceof ApiRequestError
}

export function hasErrorKey(error: unknown, key: string): boolean {
	return (
		isApiRequestError(error) && (error.code === key || error.message === key)
	)
}

const RETRYABLE_STATUSES: ReadonlySet<number> = new Set([408, 429])
export const MAX_RETRIES = 3

export function isRetryableStatus(status: number): boolean {
	return RETRYABLE_STATUSES.has(status) || (status >= 500 && status <= 599)
}

export function shouldRetry(failureCount: number, error: unknown): boolean {
	if (failureCount >= MAX_RETRIES) return false
	if (!isApiRequestError(error)) return false
	return isRetryableStatus(error.status)
}
