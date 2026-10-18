import { isApiRequestError } from '#/lib/api-error'

/** 403 : message d'admin ; 422 : le message de validation du serveur ; sinon le message ou `fallback`. */
export function mutationErrorMessage(
	error: unknown,
	forbiddenMessage: string,
	fallback: string,
): string | undefined {
	if (!error) return undefined
	if (isApiRequestError(error)) {
		if (error.status === 403) return forbiddenMessage
		return error.message || fallback
	}
	return error instanceof Error && error.message ? error.message : fallback
}
