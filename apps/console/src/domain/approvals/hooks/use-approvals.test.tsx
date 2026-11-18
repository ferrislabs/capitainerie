import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { usePendingApprovalsCount } from '#/domain/approvals/hooks/use-approvals'
import { clearAuth } from '#/store/auth.store'
import { mockApi } from '#/test-utils/api-mock'

const client = new QueryClient({
	defaultOptions: { queries: { retry: false } },
})
const wrapper = ({ children }: { children: ReactNode }) => (
	<QueryClientProvider client={client}>{children}</QueryClientProvider>
)

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('usePendingApprovalsCount', () => {
	it('reads the org pending total from the pending list', async () => {
		const fetchMock = mockApi({
			'GET /api/v1/approvals': { items: [], pending: 7 },
		})
		const { result } = renderHook(() => usePendingApprovalsCount(), {
			wrapper,
		})
		expect(result.current).toBe(0)
		await waitFor(() => expect(result.current).toBe(7))
		const url = new URL(String(fetchMock.mock.calls[0]?.[0]))
		expect(url.searchParams.get('status')).toBe('pending')
	})
})
