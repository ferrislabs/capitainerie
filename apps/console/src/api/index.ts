import { createApiClient } from '#/api/api.client'
import { fetcher } from '#/api/api.fetch'
import { TanstackQueryApiClient } from '#/api/api.tanstack'
import { getRuntimeConfig } from '#/lib/runtime-config'

export const api = createApiClient({ fetch: fetcher })
export const tanstackApi = new TanstackQueryApiClient(api)

export function syncApiBaseUrl(): void {
	const { apiUrl } = getRuntimeConfig()
	api.setBaseUrl(apiUrl || window.location.origin)
}
