import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

export const ADMIN_ONLY_GITHUB =
	'Seuls les administrateurs de l’organisation peuvent connecter ou déconnecter GitHub.'

export const NOT_OWNER_MESSAGE =
	'Vous devez être propriétaire du compte GitHub qui a installé l’application'

export function connectErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'invalid_state')) {
		return 'Le lien de connexion a expiré, recommencez'
	}
	if (
		hasErrorKey(error, 'installation_not_accessible') ||
		hasErrorKey(error, 'installation_not_owned')
	) {
		return NOT_OWNER_MESSAGE
	}
	if (hasErrorKey(error, 'installation_already_connected')) {
		return 'Cette installation GitHub est déjà connectée à une autre organisation.'
	}
	if (hasErrorKey(error, 'github_app_not_configured')) {
		return 'L’application GitHub n’est pas configurée sur ce serveur.'
	}
	if (isApiRequestError(error) && error.status === 403) return ADMIN_ONLY_GITHUB
	return 'La connexion à GitHub a échoué. Recommencez.'
}

export function managementErrorMessage(
	error: unknown,
	fallback: string,
): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'github_app_not_configured')) {
		return 'L’application GitHub n’est pas configurée sur ce serveur.'
	}
	if (isApiRequestError(error) && error.status === 403) return ADMIN_ONLY_GITHUB
	return fallback
}
