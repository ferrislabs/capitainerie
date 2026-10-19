import { isApiRequestError } from '#/lib/api-error'

export function directoryErrorMessage(error: unknown): string | undefined {
	if (!error) return undefined
	if (isApiRequestError(error)) {
		if (error.status === 409) return 'Ce nom est déjà utilisé.'
		if (error.status === 403) return 'Vous ne pouvez pas modifier cette fiche.'
		if (error.status === 404) return 'Cette fiche n’existe plus.'
	}
	return error instanceof Error ? error.message : 'Enregistrement impossible.'
}
