import { describe, expect, it } from 'vitest'
import {
	integrationErrorMessage,
	integrationFailureMessage,
} from '#/domain/clients/integration-errors'
import { ApiRequestError } from '#/lib/api-error'

describe('integrationErrorMessage', () => {
	it('quotes the server message for a connection failure', () => {
		expect(
			integrationErrorMessage(
				new ApiRequestError(422, 'connection_failed', 'timeout'),
			),
		).toBe('Connexion impossible : timeout')
	})

	it('asks to pick the other Odoo type on a version mismatch', () => {
		expect(
			integrationErrorMessage(
				new ApiRequestError(422, 'version_mismatch', 'no JSON-2 route'),
			),
		).toBe('Ce serveur ne sert pas ce protocole. Choisissez l’autre type Odoo.')
	})

	it('explains the network guard', () => {
		expect(
			integrationErrorMessage(
				new ApiRequestError(422, 'network_blocked', '10.0.0.5'),
			),
		).toBe('Adresse refusée par la garde réseau (adresse privée ou http)')
	})

	it('explains a name already used by the client', () => {
		expect(
			integrationErrorMessage(
				new ApiRequestError(409, 'instance_name_taken', 'Odoo prod'),
			),
		).toBe('Ce client a déjà une intégration de ce nom')
	})

	it('keeps the server message for the other refusals', () => {
		expect(
			integrationErrorMessage(
				new ApiRequestError(422, 'not_supported', 'n8n 1.x is not supported'),
			),
		).toBe('n8n 1.x is not supported')
		expect(
			integrationErrorMessage(
				new ApiRequestError(422, 'invalid', 'the name is empty'),
			),
		).toBe('the name is empty')
	})

	it('tells a member the action is reserved to administrators', () => {
		expect(
			integrationErrorMessage(new ApiRequestError(403, 'forbidden', 'no')),
		).toBe(
			'Seuls les administrateurs de l’organisation peuvent gérer les intégrations.',
		)
	})

	it('falls back on a generic message and says nothing without an error', () => {
		expect(integrationErrorMessage('boom')).toBe('Enregistrement impossible.')
		expect(integrationErrorMessage(null)).toBeUndefined()
	})
})

describe('integrationFailureMessage', () => {
	it('words a failed check like a refused save', () => {
		expect(integrationFailureMessage('connection_failed', 'bad key')).toBe(
			'Connexion impossible : bad key',
		)
		expect(integrationFailureMessage('something_else', 'raw')).toBe('raw')
	})
})
