import { QueryClient } from '@tanstack/react-query'
import { shouldRetry } from '#/lib/api-error'

export function createQueryClient(): QueryClient {
	return new QueryClient({
		defaultOptions: {
			queries: { retry: shouldRetry },
			mutations: { retry: false },
		},
	})
}

export function getContext() {
	return { queryClient: createQueryClient() }
}
