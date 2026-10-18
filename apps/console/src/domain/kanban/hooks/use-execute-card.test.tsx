import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useExecuteCard } from '#/domain/kanban/hooks/use-execute-card'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

describe('useExecuteCard', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		harness.seed('/api/v1/cards', { items: [] })
		harness.seed('/api/v1/runs', { items: [], total: 0 })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('posts the execution and invalidates cards and runs', async () => {
		fetchMock.mockResolvedValue(json(202, { card: { id: 'c1' }, run_id: 'r1' }))
		const { result } = renderHook(() => useExecuteCard(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { repository: 'acme/web', expected_version: 3 },
		})

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/cards/c1/executions')
		expect(init.method).toBe('POST')
		expect(JSON.parse(String(init.body))).toEqual({
			repository: 'acme/web',
			expected_version: 3,
		})
		expect(result.current.data?.run_id).toBe('r1')
		expect(harness.isInvalidated('/api/v1/cards')).toBe(true)
		expect(harness.isInvalidated('/api/v1/runs')).toBe(true)
	})

	it('maps the refusal and reloads the cards on a stale version', async () => {
		fetchMock.mockResolvedValue(
			json(409, { code: 'conflict', status: 409, message: 'stale version' }),
		)
		const { result } = renderHook(() => useExecuteCard(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({
			path: { card_id: 'c1' },
			body: { repository: 'acme/web', expected_version: 1 },
		})

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toContain('La carte a changé')
		await waitFor(() =>
			expect(harness.isInvalidated('/api/v1/cards')).toBe(true),
		)
		expect(harness.isInvalidated('/api/v1/runs')).toBe(false)
	})
})
