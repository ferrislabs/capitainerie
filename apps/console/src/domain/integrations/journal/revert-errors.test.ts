import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	restoreFailure,
	revertErrorMessage,
} from '#/domain/integrations/journal/revert-errors'
import { ApiRequestError } from '#/lib/api-error'

function row(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return {
		id: 'r1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'production',
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

describe('revertErrorMessage', () => {
	it('maps the conflicts', () => {
		const cases: Array<[string, string]> = [
			[
				'changed_since',
				'L’état a changé depuis cette action : rien n’a été restauré.',
			],
			['already_reverted', 'Cette action est déjà restaurée.'],
			['intervention_running', 'L’intervention est encore en cours.'],
		]
		for (const [code, message] of cases) {
			expect(
				revertErrorMessage(new ApiRequestError(409, code, 'english')),
			).toBe(message)
			expect(
				revertErrorMessage(new ApiRequestError(409, 'conflict', code)),
			).toBe(message)
		}
	})

	it('maps the refusals', () => {
		const cases: Array<[string, string]> = [
			['not_revertible', 'Cette action ne peut pas être restaurée.'],
			[
				'instance_archived',
				'L’instance est archivée : l’état d’avant ne peut pas y être réécrit.',
			],
			[
				'production_write_not_confirmed',
				'Cochez l’autorisation d’écriture en production.',
			],
		]
		for (const [code, message] of cases) {
			expect(
				revertErrorMessage(new ApiRequestError(422, code, 'english')),
			).toBe(message)
		}
	})

	it('reports an unreachable instance with its message', () => {
		expect(
			revertErrorMessage(
				new ApiRequestError(
					422,
					'connection_failed',
					'the instance did not answer in time',
				),
			),
		).toBe('Connexion impossible : the instance did not answer in time')
		expect(
			revertErrorMessage(new ApiRequestError(422, 'network_blocked', 'x')),
		).toBe('Adresse refusée par la garde réseau (adresse privée ou http)')
		expect(
			revertErrorMessage(new ApiRequestError(422, 'not_supported', 'x')),
		).toBe('Cette instance ne permet pas cette restauration.')
		expect(
			revertErrorMessage(new ApiRequestError(422, 'version_mismatch', 'x')),
		).toBe('Cette instance ne sert pas le protocole attendu.')
	})

	it('maps 403, 404 and unknown failures', () => {
		expect(
			revertErrorMessage(new ApiRequestError(403, 'forbidden', 'no')),
		).toBe('Vous n’avez pas le droit de restaurer cette action.')
		expect(
			revertErrorMessage(new ApiRequestError(404, 'not_found', 'no')),
		).toBe('Cette action n’existe plus.')
		expect(revertErrorMessage(new ApiRequestError(500, 'internal', 'x'))).toBe(
			'Impossible de restaurer l’état d’avant.',
		)
		expect(revertErrorMessage(new Error('boom'))).toBe(
			'Impossible de restaurer l’état d’avant.',
		)
		expect(revertErrorMessage(null)).toBeUndefined()
	})
})

describe('restoreFailure', () => {
	it('is silent when the last row succeeded', () => {
		expect(restoreFailure([row()])).toBeUndefined()
		expect(
			restoreFailure([row({ outcome: 'succeeded' }), row({ id: 'r2' })]),
		).toBeUndefined()
	})

	it('reports the reason of a last row that did not succeed', () => {
		expect(
			restoreFailure([
				row(),
				row({
					id: 'r2',
					outcome: 'failed',
					reason_code: 'connection_failed',
					reason: 'the instance did not answer in time',
				}),
			]),
		).toBe('La restauration s’est arrêtée : connexion à l’instance impossible')
	})

	it('only looks at the last row', () => {
		expect(
			restoreFailure([row({ outcome: 'failed', reason: 'early' }), row()]),
		).toBeUndefined()
	})

	it('still reports a stop without a reason, or without any row', () => {
		expect(restoreFailure([row({ outcome: 'denied' })])).toBe(
			'La restauration s’est arrêtée avant la fin.',
		)
		expect(restoreFailure([])).toBe('La restauration n’a rien écrit.')
	})
})
