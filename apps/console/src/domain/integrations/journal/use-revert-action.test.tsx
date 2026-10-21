import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { useRevertIntegrationAction } from '#/domain/integrations/journal/use-revert-action'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const CARD_ACTIONS = '/api/v1/cards/{card_id}/integration-actions'
const RUN_ACTIONS = '/api/v1/runs/{run_id}/integration-actions'

function restoreRow(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return {
		id: 'r1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'staging',
		instance_kind: 'odoo_json2',
		actor: 'ada',
		tool: 'odoo_write',
		access: 'write',
		target: {},
		arguments: {},
		outcome: 'succeeded',
		revertible: false,
		reverts_action_id: 'a1',
		created_at: '2026-10-16T09:35:00Z',
		...overrides,
	}
}

describe('useRevertIntegrationAction', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		harness.seed(CARD_ACTIONS, { items: [] })
		harness.seed(RUN_ACTIONS, { items: [] })
		harness.seed('/api/v1/cards', { items: [] })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('posts an empty body and invalidates both journals', async () => {
		fetchMock.mockResolvedValue(json(200, { items: [restoreRow()] }))
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({ path: { id: 'a1' }, body: {} })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe(
			'http://api.test/api/v1/integration-actions/a1/revert',
		)
		expect(init.method).toBe('POST')
		expect(JSON.parse(String(init.body))).toEqual({})
		expect(result.current.errorMessage).toBeUndefined()
		await waitFor(() => expect(harness.isInvalidated(CARD_ACTIONS)).toBe(true))
		expect(harness.isInvalidated(RUN_ACTIONS)).toBe(true)
		expect(harness.isInvalidated('/api/v1/cards')).toBe(false)
	})

	it('sends the production confirmation', async () => {
		fetchMock.mockResolvedValue(json(200, { items: [restoreRow()] }))
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({
			path: { id: 'a1' },
			body: { confirm_production: true },
		})

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		const [, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(JSON.parse(String(init.body))).toEqual({ confirm_production: true })
	})

	it('reads a 200 whose last row failed as a failure, and still reloads the journals', async () => {
		fetchMock.mockResolvedValue(
			json(200, {
				items: [
					restoreRow(),
					restoreRow({
						id: 'r2',
						outcome: 'failed',
						reason_code: 'connection_failed',
						reason: 'timeout',
					}),
				],
			}),
		)
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({ path: { id: 'a1' }, body: {} })

		await waitFor(() => expect(result.current.isSuccess).toBe(true))
		expect(result.current.errorMessage).toBe(
			'La restauration s’est arrêtée : connexion à l’instance impossible',
		)
		expect(result.current.restored).toBe(false)
		await waitFor(() => expect(harness.isInvalidated(CARD_ACTIONS)).toBe(true))
	})

	it('maps a refusal and reloads the journals when the action is stale', async () => {
		fetchMock.mockResolvedValue(
			json(409, {
				code: 'already_reverted',
				status: 409,
				message: 'this action is already restored',
			}),
		)
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({ path: { id: 'a1' }, body: {} })

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toBe('Cette action est déjà restaurée.')
		await waitFor(() => expect(harness.isInvalidated(CARD_ACTIONS)).toBe(true))
	})

	it('leaves the journals alone on a validation refusal', async () => {
		fetchMock.mockResolvedValue(
			json(422, {
				code: 'production_write_not_confirmed',
				status: 422,
				message: 'confirm',
			}),
		)
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})

		result.current.mutate({ path: { id: 'a1' }, body: {} })

		await waitFor(() => expect(result.current.isError).toBe(true))
		expect(result.current.errorMessage).toBe(
			'Cochez l’autorisation d’écriture en production.',
		)
		expect(harness.isInvalidated(CARD_ACTIONS)).toBe(false)
	})

	it('tells a clean restoration apart', async () => {
		fetchMock.mockResolvedValue(json(200, { items: [restoreRow()] }))
		const { result } = renderHook(() => useRevertIntegrationAction(), {
			wrapper: harness.wrapper,
		})
		expect(result.current.restored).toBe(false)

		result.current.mutate({ path: { id: 'a1' }, body: {} })

		await waitFor(() => expect(result.current.restored).toBe(true))
	})
})
