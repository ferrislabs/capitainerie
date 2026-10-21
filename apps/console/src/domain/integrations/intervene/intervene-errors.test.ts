import { describe, expect, it } from 'vitest'
import { interveneErrorMessage } from '#/domain/integrations/intervene/intervene-errors'
import { ApiRequestError } from '#/lib/api-error'

describe('interveneErrorMessage', () => {
	it('maps each refusal code to French', () => {
		const cases: Array<[string, string]> = [
			['card_without_client', 'Rattachez d’abord la carte à un client.'],
			[
				'no_instance_granted',
				'Choisissez au moins une instance, en lecture ou en écriture.',
			],
			[
				'instance_of_another_client',
				'Une instance choisie n’appartient pas au client de la carte.',
			],
			[
				'instance_archived',
				'Une instance choisie a été archivée. Fermez puis rouvrez la fenêtre pour actualiser la liste.',
			],
			[
				'production_write_not_confirmed',
				'Cochez l’autorisation d’écriture en production.',
			],
			['instructions_too_long', 'Les instructions dépassent 4 000 caractères.'],
		]
		for (const [code, message] of cases) {
			expect(
				interveneErrorMessage(new ApiRequestError(422, code, 'english')),
			).toBe(message)
		}
	})

	it('maps any other 422', () => {
		const message =
			'La demande est invalide : vérifiez les instances et les instructions.'
		expect(
			interveneErrorMessage(new ApiRequestError(422, 'invalid', 'bad')),
		).toBe(message)
		expect(
			interveneErrorMessage(new ApiRequestError(422, 'something_new', 'bad')),
		).toBe(message)
	})

	it('shares the conflict messages with the Claude Code flow', () => {
		expect(
			interveneErrorMessage(
				new ApiRequestError(409, 'conflict', 'card_not_executable'),
			),
		).toContain('« À faire »')
		expect(
			interveneErrorMessage(
				new ApiRequestError(409, 'conflict', 'execution_already_active'),
			),
		).toBe('Une exécution est déjà en cours pour cette carte.')
		expect(
			interveneErrorMessage(
				new ApiRequestError(409, 'conflict', 'version mismatch'),
			),
		).toContain('La carte a changé')
	})

	it('maps 403 and unknown failures', () => {
		expect(
			interveneErrorMessage(new ApiRequestError(403, 'forbidden', 'no')),
		).toContain('pas le droit')
		expect(interveneErrorMessage(new Error('boom'))).toBe(
			'Impossible de confier la carte.',
		)
		expect(interveneErrorMessage(null)).toBeUndefined()
	})
})
