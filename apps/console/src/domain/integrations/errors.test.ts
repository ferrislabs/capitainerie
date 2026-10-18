import { describe, expect, it } from 'vitest'
import {
	ADMIN_ONLY_GITHUB,
	connectErrorMessage,
	createAppErrorMessage,
	managementErrorMessage,
	NOT_OWNER_MESSAGE,
} from '#/domain/integrations/errors'
import { ApiRequestError } from '#/lib/api-error'

describe('connectErrorMessage', () => {
	it('maps the callback failures', () => {
		expect(
			connectErrorMessage(new ApiRequestError(400, 'invalid_state', 'x')),
		).toBe('Le lien de connexion a expiré, recommencez')
		for (const code of [
			'installation_not_accessible',
			'installation_not_owned',
		]) {
			expect(connectErrorMessage(new ApiRequestError(403, code, 'x'))).toBe(
				NOT_OWNER_MESSAGE,
			)
		}
		expect(
			connectErrorMessage(
				new ApiRequestError(409, 'conflict', 'installation_already_connected'),
			),
		).toContain('déjà connectée')
		expect(
			connectErrorMessage(new ApiRequestError(403, 'forbidden', 'nope')),
		).toBe(ADMIN_ONLY_GITHUB)
		expect(connectErrorMessage(new Error('x'))).toContain('a échoué')
		expect(connectErrorMessage(undefined)).toBeUndefined()
	})
})

describe('createAppErrorMessage', () => {
	it('maps the creation failures', () => {
		expect(
			createAppErrorMessage(new ApiRequestError(400, 'invalid_state', 'x')),
		).toContain('a expiré')
		expect(
			createAppErrorMessage(
				new ApiRequestError(409, 'conflict', 'github_app_exists'),
			),
		).toContain('déjà une application')
		expect(
			createAppErrorMessage(new ApiRequestError(422, 'invalid', 'x')),
		).toContain('GitHub a refusé')
		expect(
			createAppErrorMessage(new ApiRequestError(403, 'forbidden', 'x')),
		).toBe(ADMIN_ONLY_GITHUB)
		expect(createAppErrorMessage(new Error('x'))).toContain('a échoué')
		expect(createAppErrorMessage(undefined)).toBeUndefined()
	})
})

describe('managementErrorMessage', () => {
	it('maps 403, the missing console URL and the missing App, otherwise the fallback', () => {
		expect(
			managementErrorMessage(new ApiRequestError(403, 'forbidden', 'x'), 'f'),
		).toBe(ADMIN_ONLY_GITHUB)
		expect(
			managementErrorMessage(
				new ApiRequestError(503, 'console_url_not_configured', 'x'),
				'f',
			),
		).toContain('CONSOLE_URL')
		expect(
			managementErrorMessage(
				new ApiRequestError(409, 'conflict', 'github_app_missing'),
				'f',
			),
		).toContain('Créez d’abord')
		expect(
			managementErrorMessage(new ApiRequestError(422, 'invalid', 'x'), 'f'),
		).toContain('login')
		expect(managementErrorMessage(new Error('x'), 'f')).toBe('f')
	})
})
