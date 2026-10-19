import { isApiRequestError } from '#/lib/api-error'

export const READER_MISSING_MESSAGE =
	'Aucun lecteur de PDF n’est configuré : choisissez-en un dans Paramètres › Modèles.'

export function documentErrorMessage(error: unknown): string {
	if (!isApiRequestError(error)) {
		return 'Impossible de joindre l’API pour lire ce PDF.'
	}
	if (error.code === 'reader_not_configured') {
		return READER_MISSING_MESSAGE
	}
	switch (error.status) {
		case 413:
			return 'Le fichier dépasse 25 Mo.'
		case 415:
			return 'Ce fichier n’est pas un PDF valide.'
		default:
			return error.message
	}
}
