import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

export const ADMIN_ONLY_GITHUB =
	'Seuls les administrateurs de l’organisation peuvent gérer GitHub.'

export const NOT_OWNER_MESSAGE =
	'Vous devez être propriétaire du compte GitHub qui a installé l’application'

const CONSOLE_URL_MISSING =
	'L’adresse de la console n’est pas configurée sur le serveur (CONSOLE_URL).'

const APP_MISSING = 'Créez d’abord l’application GitHub de l’organisation.'

const APP_EXISTS = 'Cette organisation a déjà une application GitHub.'

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
	if (hasErrorKey(error, 'github_app_members_permission_missing')) {
		return 'L’application GitHub doit pouvoir lire les membres de l’organisation : dans ses réglages GitHub, Permissions › Organization permissions › Members : Read-only. Acceptez la nouvelle permission sur l’installation, puis recommencez.'
	}
	if (hasErrorKey(error, 'installation_already_connected')) {
		return 'Cette installation GitHub est déjà connectée à une autre organisation.'
	}
	if (hasErrorKey(error, 'github_app_missing')) return APP_MISSING
	if (isApiRequestError(error) && error.status === 403) return ADMIN_ONLY_GITHUB
	return 'La connexion à GitHub a échoué. Recommencez.'
}

export function createAppErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'invalid_state')) {
		return 'Le lien de création a expiré, recommencez depuis les intégrations.'
	}
	if (hasErrorKey(error, 'github_app_exists')) return APP_EXISTS
	if (isApiRequestError(error)) {
		if (error.status === 403) return ADMIN_ONLY_GITHUB
		if (error.status === 422) {
			return 'GitHub a refusé la création de l’application (code expiré ou déjà utilisé). Recommencez depuis les intégrations.'
		}
	}
	return 'La création de l’application GitHub a échoué. Recommencez.'
}

export function managementErrorMessage(
	error: unknown,
	fallback: string,
): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'console_url_not_configured')) {
		return CONSOLE_URL_MISSING
	}
	if (hasErrorKey(error, 'github_app_missing')) return APP_MISSING
	if (hasErrorKey(error, 'github_app_exists')) return APP_EXISTS
	if (isApiRequestError(error)) {
		if (error.status === 403) return ADMIN_ONLY_GITHUB
		if (error.status === 422) {
			return 'Le login de l’organisation GitHub n’est pas valide.'
		}
	}
	return fallback
}
