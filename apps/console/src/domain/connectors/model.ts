function statusOf(error: unknown): number | undefined {
	return (error as { status?: number } | null)?.status
}

const NOT_CONFIGURED =
	"Les connecteurs MCP ne sont pas configurés sur ce serveur. Contacte l'administrateur."
const FORBIDDEN = "Tu n'as pas le droit de gérer les connecteurs."

export function describeCreateError(error: unknown): string {
	switch (statusOf(error)) {
		case 409:
			return 'Un connecteur utilise déjà cet identifiant. Choisis un autre nom.'
		case 422:
			return "URL MCP invalide ou injoignable : le serveur n'a pas répondu comme un serveur MCP OAuth2."
		case 503:
			return NOT_CONFIGURED
		case 403:
			return FORBIDDEN
		default:
			return "L'ajout du connecteur a échoué. Réessaie."
	}
}

export function describeConnectError(error: unknown): string {
	switch (statusOf(error)) {
		case 404:
			return "Ce connecteur n'existe plus."
		case 409:
			return 'Ce connecteur est archivé : restaure-le avant de te connecter.'
		case 503:
			return NOT_CONFIGURED
		default:
			return 'La connexion a échoué. Réessaie.'
	}
}

export function describeTestError(error: unknown): string {
	switch (statusOf(error)) {
		case 404:
			return "Ce connecteur n'existe plus."
		case 409:
			return "Tu n'es pas connecté à ce connecteur : connecte-toi d'abord."
		case 422:
			return 'Le serveur MCP a refusé la requête ou a répondu de façon invalide. Reconnecte-toi puis réessaie.'
		case 503:
			return NOT_CONFIGURED
		default:
			return 'Le test a échoué. Réessaie.'
	}
}

export function describeUpdateError(error: unknown): string {
	switch (statusOf(error)) {
		case 404:
			return "Ce connecteur n'existe plus."
		case 403:
			return FORBIDDEN
		case 422:
			return 'Modification refusée : le nom est invalide.'
		default:
			return 'La modification a échoué. Réessaie.'
	}
}

export function describeDisconnectError(error: unknown): string {
	return statusOf(error) === 404
		? "Ce connecteur n'existe plus."
		: 'La déconnexion a échoué. Réessaie.'
}

export function describeCallbackError(error: unknown): string {
	switch (statusOf(error)) {
		case 422:
			return "Le retour d'autorisation est invalide ou a expiré. Relance la connexion depuis la page des connecteurs."
		case 503:
			return NOT_CONFIGURED
		case 403:
			return FORBIDDEN
		default:
			return "La connexion n'a pas pu être finalisée. Relance-la depuis la page des connecteurs."
	}
}

export function hostOf(url: string): string {
	try {
		return new URL(url).host
	} catch {
		return url
	}
}

export function expiryLabel(
	expiresAt: string | null | undefined,
	now: Date = new Date(),
): { text: string; expired: boolean } | null {
	if (!expiresAt) return null
	const date = new Date(expiresAt)
	if (Number.isNaN(date.getTime())) return null
	const text = date.toLocaleDateString('fr-FR', {
		day: 'numeric',
		month: 'short',
		year: 'numeric',
	})
	return date < now
		? { text: `Expiré le ${text}`, expired: true }
		: { text: `Expire le ${text}`, expired: false }
}
