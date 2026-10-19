import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { useCaptainModel } from '#/domain/chat/hooks/use-captain-model'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const ROLES = {
	extractor: { provider: 'mistral', model: 'm' },
	judge: { provider: 'mistral', model: 'm' },
	writer: { provider: 'mistral', model: 'mistral-small' },
	embeddings: { provider: 'mistral', model: 'e' },
}

describe('useCaptainModel', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	function modelFor(assignments: unknown) {
		fetchMock.mockResolvedValue(json(200, { assignments }))
		return renderHook(() => useCaptainModel(), { wrapper: harness.wrapper })
	}

	it('is loading first', () => {
		const { result } = modelFor(ROLES)

		expect(result.current).toEqual({ kind: 'loading' })
	})

	it('follows the writer when no Captain is chosen', async () => {
		const { result } = modelFor({ ...ROLES, captain: null })

		await waitFor(() =>
			expect(result.current).toEqual({ kind: 'ready', model: 'mistral-small' }),
		)
	})

	it('prefers the chosen Captain', async () => {
		const { result } = modelFor({
			...ROLES,
			captain: { provider: 'anthropic', model: 'claude-sonnet-5-5' },
		})

		await waitFor(() =>
			expect(result.current).toEqual({
				kind: 'ready',
				model: 'claude-sonnet-5-5',
			}),
		)
	})

	it('is unknown when no model is assigned yet', async () => {
		const { result } = modelFor(null)

		await waitFor(() => expect(result.current).toEqual({ kind: 'unknown' }))
	})
})
