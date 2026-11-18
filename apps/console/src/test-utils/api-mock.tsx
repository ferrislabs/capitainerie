import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { vi } from 'vitest'
import { api } from '#/api'
import { setAuth } from '#/store/auth.store'

export interface MockReply {
	status: number
	body: unknown
}

export type MockRoutes = Readonly<Record<string, MockReply | unknown>>

const isReply = (value: unknown): value is MockReply =>
	typeof value === 'object' &&
	value !== null &&
	'status' in value &&
	'body' in value

export function mockApi(routes: MockRoutes) {
	api.setBaseUrl('http://api.test')
	setAuth({ accessToken: 'tok', isAuthenticated: true })
	const fetchMock = vi.fn(async (input: URL | string, init?: RequestInit) => {
		const url = new URL(String(input))
		const key = `${init?.method ?? 'GET'} ${url.pathname}`
		const route = routes[key]
		if (route === undefined) {
			return new Response(
				JSON.stringify({ code: 'not_found', message: key, status: 404 }),
				{ status: 404, headers: { 'Content-Type': 'application/json' } },
			)
		}
		const reply = isReply(route) ? route : { status: 200, body: route }
		return new Response(JSON.stringify(reply.body), {
			status: reply.status,
			headers: { 'Content-Type': 'application/json' },
		})
	})
	vi.stubGlobal('fetch', fetchMock)
	return fetchMock
}

export function withQueryClient(ui: ReactNode) {
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
	})
	return <QueryClientProvider client={client}>{ui}</QueryClientProvider>
}
