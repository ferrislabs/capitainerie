import { describe, expect, it } from 'vitest'
import {
	ADMIN_ONLY_GITHUB,
	connectErrorMessage,
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

describe('managementErrorMessage', () => {
	it('maps 403 and unconfigured, otherwise the fallback', () => {
		expect(
			managementErrorMessage(new ApiRequestError(403, 'forbidden', 'x'), 'f'),
		).toBe(ADMIN_ONLY_GITHUB)
		expect(
			managementErrorMessage(
				new ApiRequestError(503, 'github_app_not_configured', 'x'),
				'f',
			),
		).toContain('pas configurée')
		expect(managementErrorMessage(new Error('x'), 'f')).toBe('f')
	})
})
