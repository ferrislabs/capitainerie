import type { Schemas } from '#/api/api.client'

export type TeamAgent = Schemas.TeamAgentResponse

export function describeAgentError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 409) return 'Un agent porte déjà ce nom.'
	if (status === 403) return "Tu n'as pas le droit de modifier les agents."
	if (status === 404) return "Cet agent n'existe plus."
	if (status === 422)
		return "Modification refusée : un agent ne peut pas dépendre de lui-même ni de l'un de ses sous-agents, et les informations doivent être valides."
	return "L'enregistrement a échoué. Réessaie."
}
