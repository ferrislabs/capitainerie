import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { canRevert } from '#/domain/integrations/journal/revert-rules'

function action(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return {
		id: 'a1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'production',
		instance_kind: 'odoo_json2',
		actor: 'capitainerie-agent-integrator',
		tool: 'odoo_write',
		access: 'write',
		target: {},
		arguments: {},
		outcome: 'succeeded',
		revertible: true,
		created_at: '2026-10-16T09:31:00Z',
		...overrides,
	}
}

describe('canRevert', () => {
	it('offers a restoration on a revertible, succeeded, untouched action', () => {
		expect(canRevert(action())).toBe(true)
		expect(
			canRevert(
				action({ reverted_by_action_id: null, reverts_action_id: null }),
			),
		).toBe(true)
	})

	it('refuses an action that is not revertible', () => {
		expect(canRevert(action({ revertible: false }))).toBe(false)
	})

	it('refuses an action that did not succeed', () => {
		for (const outcome of ['pending', 'denied', 'failed'] as const) {
			expect(canRevert(action({ outcome }))).toBe(false)
		}
	})

	it('refuses an action that is already restored', () => {
		expect(canRevert(action({ reverted_by_action_id: 'a2' }))).toBe(false)
	})

	it('refuses an action that is itself a restoration', () => {
		expect(canRevert(action({ reverts_action_id: 'a0' }))).toBe(false)
	})
})
