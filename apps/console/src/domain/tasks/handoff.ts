import type { Schemas } from '#/api/api.client'

export type AgentRunOutcome = Schemas.AgentRunOutcome

/** Ce que la personne lit quand l'agent n'a pas démarré ; rien quand il a démarré. */
export function handoffNotice(
	outcome: AgentRunOutcome | null | undefined,
): string | null {
	if (!outcome || outcome.run_id) return null
	if (
		outcome.skipped === 'agent_busy' ||
		outcome.skipped === 'choose_repository'
	)
		return outcome.message ?? null
	return `L'agent n'a pas démarré : ${outcome.message ?? 'raison inconnue'}`
}

/**
 * La création d'une tâche renvoie le démarrage de l'agent, mais la fenêtre se
 * ferme et la page de la tâche s'ouvre : elle le reprend ici, une seule fois.
 */
const pending = new Map<string, AgentRunOutcome>()

export function rememberHandoff(
	cardId: string,
	outcome: AgentRunOutcome | null | undefined,
): void {
	if (outcome) pending.set(cardId, outcome)
}

export function takeHandoff(cardId: string): AgentRunOutcome | undefined {
	const outcome = pending.get(cardId)
	pending.delete(cardId)
	return outcome
}
