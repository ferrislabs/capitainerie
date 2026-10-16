import { ApiRequestError } from '#/lib/api-error'
import { getRuntimeConfig } from '#/lib/runtime-config'
import { authStore } from '#/store/auth.store'

export interface FetcherInput {
	method: string
	url: URL
	urlSearchParams?: URLSearchParams
	parameters?: {
		body?: unknown
		header?: Record<string, unknown>
	}
	overrides?: RequestInit
}

function isRawBody(body: unknown): body is BodyInit {
	return (
		body instanceof URLSearchParams ||
		body instanceof FormData ||
		typeof body === 'string' ||
		body instanceof Blob ||
		body instanceof ArrayBuffer
	)
}

async function toApiError(response: Response): Promise<ApiRequestError> {
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

export async function fetcher(input: FetcherInput): Promise<Response> {
	const headers = new Headers()
	const accessToken = authStore.state.accessToken

	if (input.urlSearchParams) {
		input.url.search = input.urlSearchParams.toString()
	}

	let body: BodyInit | undefined
	const method = input.method.toUpperCase()
	const bodyData = input.parameters?.body
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

	if (accessToken) {
		headers.set('Authorization', `Bearer ${accessToken}`)
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

export async function apiRequest<T>(
	path: string,
	init: { method?: string; body?: unknown } = {},
): Promise<T> {
	const url = new URL(path, getRuntimeConfig().apiUrl)
	const response = await fetcher({
		method: init.method ?? 'GET',
		url,
		parameters: { body: init.body },
	})
	if (response.status === 204) return undefined as T
	return (await response.json()) as T
}
