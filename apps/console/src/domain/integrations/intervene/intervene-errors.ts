import { executionErrorMessage } from '#/domain/kanban/execution-errors'
import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

const REFUSALS: ReadonlyArray<readonly [string, string]> = [
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

export function interveneErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	for (const [code, message] of REFUSALS) {
		if (hasErrorKey(error, code)) return message
	}
	if (isApiRequestError(error) && error.status === 422) {
		return 'La demande est invalide : vérifiez les instances et les instructions.'
	}
	return executionErrorMessage(error)
}
