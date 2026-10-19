import type { Fetcher } from '#/api/api.client'
import { ApiRequestError } from '#/lib/api-error'
import { authStore } from '#/store/auth.store'

function isRawBody(body: unknown): body is BodyInit {
	return (
		body instanceof URLSearchParams ||
		body instanceof FormData ||
		typeof body === 'string' ||
		body instanceof Blob ||
		body instanceof ArrayBuffer
	)
}

export async function toApiError(response: Response): Promise<ApiRequestError> {
	let body: { code?: unknown; message?: unknown } | undefined
	try {
		body = await response.json()
	} catch {
		body = undefined
	}
	const code =
		typeof body?.code === 'string' ? body.code : `http_${response.status}`
	const message =
		typeof body?.message === 'string'
			? body.message
			: `HTTP ${response.status}: ${response.statusText}`
	return new ApiRequestError(response.status, code, message)
}

export function authHeaders(): Headers {
	const headers = new Headers()
	const accessToken = authStore.state.accessToken
	if (accessToken) {
		headers.set('Authorization', `Bearer ${accessToken}`)
	}
	return headers
}

export const fetcher: Fetcher['fetch'] = async (input) => {
	const headers = authHeaders()

	if (input.urlSearchParams) {
		input.url.search = input.urlSearchParams.toString()
	}

	let body: BodyInit | undefined
	const method = input.method.toUpperCase()
	const bodyData =
		input.parameters && 'body' in input.parameters
			? input.parameters.body
			: undefined
	if (
		['POST', 'PUT', 'PATCH', 'DELETE'].includes(method) &&
		bodyData !== undefined
	) {
		if (isRawBody(bodyData)) {
			body = bodyData
		} else {
			body = JSON.stringify(bodyData)
			headers.set('Content-Type', 'application/json')
		}
	}

	if (input.parameters?.header) {
		for (const [key, value] of Object.entries(input.parameters.header)) {
			if (value != null) headers.set(key, String(value))
		}
	}

	const response = await fetch(input.url, {
		method,
		...(body !== undefined && { body }),
		headers,
		...input.overrides,
	})

	if (!response.ok) {
		throw await toApiError(response)
	}

	return response
}
