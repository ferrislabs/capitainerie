import { isApiRequestError } from '#/lib/api-error'

export const CAPTAIN_MISSING_MESSAGE =
	'Aucun modèle ne sert le Capitaine : choisissez-en un dans Paramètres › Modèles.'

export const CONVERSATION_FULL_MESSAGE =
	'Cette conversation a atteint sa limite : ouvrez-en une nouvelle.'

export function chatErrorMessage(error: unknown): string {
	if (!isApiRequestError(error)) {
		return 'Impossible de joindre l’API pour parler au Capitaine.'
	}
	switch (error.code) {
		case 'turn_in_progress':
			return 'Le Capitaine répond déjà dans cette conversation : attendez la fin de sa réponse.'
		case 'conversation_full':
			return CONVERSATION_FULL_MESSAGE
		case 'captain_not_configured':
			return CAPTAIN_MISSING_MESSAGE
		case 'captain_unavailable':
			return 'Le Capitaine ne peut pas agir pour vous : l’échange de jeton avec FerrisKey n’est pas configuré ou a été refusé.'
		case 'provider_error':
			return `Le modèle n’a pas pu répondre : ${error.message}`
		default:
			break
	}
	if (error.status === 403) {
		return 'Les agents ne peuvent pas utiliser le chat.'
	}
	if (error.status === 404) return 'Cette conversation est introuvable.'
	return error.message
}

export function streamErrorMessage(code: string, message: string): string {
	if (code === 'provider_error') {
		return `Le modèle n’a pas pu répondre : ${message}`
	}
	if (code === 'storage_failed') {
		return 'La réponse n’a pas pu être enregistrée.'
	}
	return message || 'Le Capitaine a rencontré une erreur.'
}
