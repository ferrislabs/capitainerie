import { isApiRequestError } from '#/lib/api-error'

export const TRANSCRIBER_MISSING_MESSAGE =
	'Aucun transcripteur n’est configuré : choisissez-en un dans Paramètres › Modèles.'

export function transcriptionErrorMessage(error: unknown): string {
	if (!isApiRequestError(error)) {
		return 'Impossible de joindre l’API pour transcrire ce fichier.'
	}
	if (error.code === 'transcriber_not_configured') {
		return TRANSCRIBER_MISSING_MESSAGE
	}
	switch (error.status) {
		case 413:
			return 'Le fichier dépasse 25 Mo.'
		case 415:
			return 'Format audio non pris en charge : utilisez un fichier .mp3, .m4a, .wav, .webm ou .ogg.'
		case 502:
			return `Le fournisseur n’a pas pu transcrire ce fichier : ${error.message}`
		default:
			return error.message
	}
}
