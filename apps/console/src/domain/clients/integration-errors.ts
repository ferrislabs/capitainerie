import { isApiRequestError } from '#/lib/api-error'

const ADMIN_ONLY =
	'Seuls les administrateurs de l’organisation peuvent gérer les intégrations.'

export function integrationFailureMessage(
	code: string,
	message: string,
): string {
	switch (code) {
		case 'connection_failed':
			return `Connexion impossible : ${message}`
		case 'version_mismatch':
			return 'Ce serveur ne sert pas ce protocole. Choisissez l’autre type Odoo.'
		case 'network_blocked':
			return 'Adresse refusée par la garde réseau (adresse privée ou http)'
		case 'instance_name_taken':
			return 'Ce client a déjà une intégration de ce nom'
		case 'instance_in_use':
			return 'Une intervention en cours utilise cette intégration : attendez sa fin pour l’archiver.'
		default:
			return message
	}
}

export function integrationErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (isApiRequestError(error)) {
		if (error.status === 403) return ADMIN_ONLY
		return integrationFailureMessage(error.code, error.message)
	}
	return error instanceof Error ? error.message : 'Enregistrement impossible.'
}
