import type { Schemas } from '#/api/api.client'
import { integrationFailureMessage } from '#/domain/clients/integration-errors'
import { reasonLabel } from '#/domain/integrations/journal/action-labels'
import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

const REFUSALS: ReadonlyArray<readonly [string, string]> = [
	[
		'changed_since',
		'L’état a changé depuis cette action : rien n’a été restauré.',
	],
	['already_reverted', 'Cette action est déjà restaurée.'],
	['intervention_running', 'L’intervention est encore en cours.'],
	['not_revertible', 'Cette action ne peut pas être restaurée.'],
	[
		'instance_archived',
		'L’instance est archivée : l’état d’avant ne peut pas y être réécrit.',
	],
	[
		'production_write_not_confirmed',
		'Cochez l’autorisation d’écriture en production.',
	],
	['not_supported', 'Cette instance ne permet pas cette restauration.'],
	['version_mismatch', 'Cette instance ne sert pas le protocole attendu.'],
]

const UNREACHABLE = ['connection_failed', 'network_blocked']

export function revertErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	for (const [code, message] of REFUSALS) {
		if (hasErrorKey(error, code)) return message
	}
	if (isApiRequestError(error)) {
		for (const code of UNREACHABLE) {
			if (hasErrorKey(error, code)) {
				return integrationFailureMessage(code, error.message)
			}
		}
		if (error.status === 403) {
			return 'Vous n’avez pas le droit de restaurer cette action.'
		}
		if (error.status === 404) return 'Cette action n’existe plus.'
	}
	return 'Impossible de restaurer l’état d’avant.'
}

export function restoreFailure(
	rows: ReadonlyArray<Schemas.IntegrationActionResponse>,
): string | undefined {
	const last = rows.at(-1)
	if (!last) return 'La restauration n’a rien écrit.'
	if (last.outcome === 'succeeded') return undefined
	const reason = reasonLabel(last.reason_code, last.reason)
	return reason
		? `La restauration s’est arrêtée : ${reason}`
		: 'La restauration s’est arrêtée avant la fin.'
}
