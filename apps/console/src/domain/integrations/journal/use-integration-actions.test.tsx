import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import {
	JOURNAL_POLL_INTERVAL_MS,
	journalRefetchInterval,
	useCardIntegrationActions,
	useRunIntegrationActions,
} from '#/domain/integrations/journal/use-integration-actions'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

describe('journalRefetchInterval', () => {
	it('polls every 5 seconds while an intervention is active, never otherwise', () => {
		expect(JOURNAL_POLL_INTERVAL_MS).toBe(5000)
		expect(journalRefetchInterval(true)).toBe(5000)
		expect(journalRefetchInterval(false)).toBe(false)
	})
})

describe('integration actions hooks', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function calls(path: string) {
		return fetchMock.mock.calls.filter(
			([url]) => (url as URL).pathname === path,
		).length
	}

	function refetchInterval(path: string) {
		const query = harness.client
			.getQueryCache()
			.getAll()
			.find((candidate) => {
				const head = candidate.queryKey[0] as { _id?: string }
				return head._id === path
			})
		return query?.observers[0]?.options.refetchInterval
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		fetchMock.mockImplementation(() =>
			Promise.resolve(json(200, { items: [] })),
		)
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('reads the journal of a card', async () => {
		const { result } = renderHook(
			() => useCardIntegrationActions('c1', false),
			{
				wrapper: harness.wrapper,
			},
		)

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		expect(calls('/api/v1/cards/c1/integration-actions')).toBe(1)
	})

	it('reads the journal of a run', async () => {
		const { result } = renderHook(() => useRunIntegrationActions('r1', false), {
			wrapper: harness.wrapper,
		})

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		expect(calls('/api/v1/runs/r1/integration-actions')).toBe(1)
	})

	it('does not fetch without an id', () => {
		renderHook(() => useCardIntegrationActions(undefined, false), {
			wrapper: harness.wrapper,
		})
		renderHook(() => useRunIntegrationActions(undefined, false), {
			wrapper: harness.wrapper,
		})

		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('polls only while asked to, and reads once more when it stops', async () => {
		const { result, rerender } = renderHook(
			({ polling }) => useCardIntegrationActions('c1', polling),
			{ wrapper: harness.wrapper, initialProps: { polling: true } },
		)
		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		expect(refetchInterval('/api/v1/cards/{card_id}/integration-actions')).toBe(
			5000,
		)
		expect(calls('/api/v1/cards/c1/integration-actions')).toBe(1)

		rerender({ polling: false })

		await waitFor(() =>
			expect(calls('/api/v1/cards/c1/integration-actions')).toBe(2),
		)
		expect(refetchInterval('/api/v1/cards/{card_id}/integration-actions')).toBe(
			false,
		)
	})

	it('does not read again when it never polled', async () => {
		const { result, rerender } = renderHook(
			({ polling }) => useRunIntegrationActions('r1', polling),
			{ wrapper: harness.wrapper, initialProps: { polling: false } },
		)
		await waitFor(() => expect(result.current.isSuccess).toBe(true))

		rerender({ polling: false })

		expect(calls('/api/v1/runs/r1/integration-actions')).toBe(1)
	})
})
