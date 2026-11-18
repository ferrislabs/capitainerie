import { hasErrorKey, isApiRequestError } from '#/lib/api-error'

export const ISSUES_PERMISSION_MESSAGE =
	"L'application GitHub doit pouvoir lire les issues : dans ses réglages GitHub, Permissions › Repository permissions › Issues : Read-only. Accepte la nouvelle permission sur l'installation, puis recharge."

export function launchErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (hasErrorKey(error, 'github_not_connected')) {
		return "GitHub n'est pas connecté à l'organisation."
	}
	if (hasErrorKey(error, 'repository_not_allowed')) {
		return "L'application GitHub n'a pas accès à ce dépôt. Choisis-en un autre."
	}
	if (hasErrorKey(error, 'issue_already_imported')) {
		return "Une des issues a déjà un chantier. Rien n'a été lancé : décoche-la et recommence."
	}
	if (hasErrorKey(error, 'github_app_issues_permission_missing')) {
		return ISSUES_PERMISSION_MESSAGE
	}
	if (isApiRequestError(error)) {
		if (error.status === 404) return "Ce projet n'existe plus."
		if (error.status === 403) {
			return "Tu n'as pas le droit de lancer ces chantiers."
		}
		if (error.status === 422) {
			return error.message.startsWith('issue #')
				? `${error.message.replace('is not open', "n'est plus ouverte")}. Rien n'a été lancé.`
				: 'La demande est invalide : vérifie le dépôt, les prompts et les issues choisis.'
		}
		return error.message || 'Impossible de lancer les chantiers.'
	}
	return 'Impossible de lancer les chantiers.'
}

export function issuesErrorMessage(error: unknown): string {
	if (hasErrorKey(error, 'github_app_issues_permission_missing')) {
		return ISSUES_PERMISSION_MESSAGE
	}
	if (hasErrorKey(error, 'repository_not_allowed')) {
		return "L'application GitHub n'a pas accès à ce dépôt."
	}
	return 'Impossible de charger les issues de ce dépôt.'
}
