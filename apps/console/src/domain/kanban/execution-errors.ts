import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

export const EXECUTION_CONFLICT_MESSAGE =
	'La carte a changé depuis son affichage. Elle vient d’être rechargée, réessayez.'

export function executionErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'card_not_executable')) {
		return 'Cette carte ne peut pas être confiée : elle doit être en « À faire ».'
	}
	if (hasErrorKey(error, 'execution_already_active')) {
		return 'Une exécution est déjà en cours pour cette carte.'
	}
	if (hasErrorKey(error, 'github_not_connected')) {
		return 'GitHub n’est pas connecté à l’organisation.'
	}
	if (hasErrorKey(error, 'repository_not_allowed')) {
		return 'L’application GitHub n’a pas accès à ce dépôt. Choisissez-en un autre.'
	}
	if (isApiRequestError(error)) {
		if (error.status === 409) return EXECUTION_CONFLICT_MESSAGE
		if (error.status === 403) {
			return 'Vous n’avez pas le droit de confier cette carte.'
		}
		if (error.status === 422) {
			return 'La demande est invalide : vérifiez le dépôt choisi.'
		}
		return error.message || 'Impossible de confier la carte.'
	}
	return 'Impossible de confier la carte.'
}

export function isVersionConflict(error: unknown): boolean {
	return (
		isApiRequestError(error) &&
		error.status === 409 &&
		!hasErrorKey(error, 'card_not_executable') &&
		!hasErrorKey(error, 'execution_already_active') &&
		!hasErrorKey(error, 'github_not_connected')
	)
}
