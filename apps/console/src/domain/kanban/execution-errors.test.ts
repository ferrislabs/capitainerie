import { describe, expect, it } from 'vitest'
import {
	EXECUTION_CONFLICT_MESSAGE,
	executionErrorMessage,
	isVersionConflict,
} from '#/domain/kanban/execution-errors'
import { ApiRequestError } from '#/lib/api-error'

describe('executionErrorMessage', () => {
	it('maps the conflict codes carried by the message', () => {
		expect(
			executionErrorMessage(
				new ApiRequestError(409, 'conflict', 'card_not_executable'),
			),
		).toContain('« À faire »')
		expect(
			executionErrorMessage(
				new ApiRequestError(409, 'conflict', 'execution_already_active'),
			),
		).toBe('Une exécution est déjà en cours pour cette carte.')
		expect(
			executionErrorMessage(
				new ApiRequestError(409, 'conflict', 'github_not_connected'),
			),
		).toBe('GitHub n’est pas connecté à l’organisation.')
	})

	it('treats any other 409 as a stale card', () => {
		const error = new ApiRequestError(409, 'conflict', 'version mismatch')
		expect(executionErrorMessage(error)).toBe(EXECUTION_CONFLICT_MESSAGE)
		expect(isVersionConflict(error)).toBe(true)
		expect(
			isVersionConflict(
				new ApiRequestError(409, 'conflict', 'card_not_executable'),
			),
		).toBe(false)
	})

	it('maps 403, 422 and unknown failures', () => {
		expect(
			executionErrorMessage(
				new ApiRequestError(403, 'forbidden', 'repository_not_allowed'),
			),
		).toContain('n’a pas accès à ce dépôt')
		expect(
			executionErrorMessage(new ApiRequestError(403, 'forbidden', 'nope')),
		).toContain('pas le droit')
		expect(
			executionErrorMessage(new ApiRequestError(422, 'invalid', 'bad repo')),
		).toBe('La demande est invalide : vérifiez le dépôt choisi.')
		expect(executionErrorMessage(new Error('boom'))).toBe(
			'Impossible de confier la carte.',
		)
		expect(executionErrorMessage(null)).toBeUndefined()
	})
})
