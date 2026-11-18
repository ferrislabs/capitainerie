import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

const FORBIDDEN =
	"Seul un administrateur de l'organisation peut configurer GitHub."
const NOT_CONFIGURED =
	"Le serveur n'est pas configuré pour GitHub (CONSOLE_URL ou SECRETS_KEY manquant). Contacte l'administrateur."
const RESTART =
	'Le lien a expiré ou ne vient pas de toi. Recommence depuis la page GitHub.'

function statusOf(error: unknown): number | undefined {
	return isApiRequestError(error) ? error.status : undefined
}

function common(error: unknown): string | undefined {
	if (hasErrorKey(error, 'invalid_state')) return RESTART
	if (hasErrorKey(error, 'github_app_exists')) {
		return "L'organisation a déjà une application GitHub."
	}
	if (hasErrorKey(error, 'github_app_missing')) {
		return "L'organisation n'a pas encore d'application GitHub : crée-la d'abord."
	}
	const status = statusOf(error)
	if (status === 503) return NOT_CONFIGURED
	if (status === 403) return FORBIDDEN
	return undefined
}

export function describeManifestError(error: unknown): string {
	return (
		common(error) ??
		(statusOf(error) === 422
			? "Ce nom d'organisation GitHub n'est pas valide."
			: "La création de l'application n'a pas pu démarrer. Réessaie.")
	)
}

export function describeCreateAppError(error: unknown): string {
	return (
		common(error) ??
		(statusOf(error) === 422
			? 'GitHub a refusé le code : il a expiré ou a déjà servi. Recommence la création.'
			: "L'application n'a pas pu être enregistrée. Recommence la création.")
	)
}

export function describeInstallError(error: unknown): string {
	return (
		common(error) ?? "Le lien d'installation n'a pas pu être obtenu. Réessaie."
	)
}

export function describeConnectError(error: unknown): string {
	if (hasErrorKey(error, 'installation_not_accessible')) {
		return "Ton compte GitHub n'a pas accès à cette installation."
	}
	if (hasErrorKey(error, 'installation_not_owned')) {
		return "Tu n'administres pas le compte GitHub sur lequel l'application est installée."
	}
	if (hasErrorKey(error, 'github_app_members_permission_missing')) {
		return "L'application GitHub doit pouvoir lire les membres de l'organisation : dans ses réglages GitHub, Permissions › Organization permissions › Members : Read-only, puis accepte la permission sur l'installation."
	}
	if (hasErrorKey(error, 'installation_already_connected')) {
		return 'Cette installation est déjà reliée à une autre organisation.'
	}
	return (
		common(error) ??
		"La connexion n'a pas pu être finalisée. Recommence l'installation."
	)
}

export function describeDisconnectError(error: unknown): string {
	return common(error) ?? 'La déconnexion a échoué. Réessaie.'
}

export function describeDeleteAppError(error: unknown): string {
	if (statusOf(error) === 404) return "L'application n'existe déjà plus."
	return common(error) ?? "La suppression de l'application a échoué. Réessaie."
}
