import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'

export function json(status: number, body: unknown) {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' },
	})
}

export function createTestClient() {
	const client = new QueryClient({
		defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
	})
	function wrapper({ children }: { children: ReactNode }) {
		return <QueryClientProvider client={client}>{children}</QueryClientProvider>
	}
	function seed(path: string, data: unknown) {
		client.setQueryData([{ _id: path }], data)
	}
	function isInvalidated(path: string) {
		return client.getQueryState([{ _id: path }])?.isInvalidated
	}
	return { client, wrapper, seed, isInvalidated }
}
