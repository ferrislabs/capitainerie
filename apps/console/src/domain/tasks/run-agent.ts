import type { Schemas } from '#/api/api.client'

export type RunStatus = Schemas.RunStatusResponse

export const RUN_STATUS_LABEL: Record<RunStatus, string> = {
	queued: 'En file d’attente',
	running: 'En cours',
	succeeded: 'Terminée',
	failed: 'Échec',
	cancelled: 'Annulée',
}

export const RUN_POLL_MS = 2000

export function isActiveRun(status: RunStatus): boolean {
	return status === 'queued' || status === 'running'
}

export function canRunAgent(column: string, hasAgent: boolean): boolean {
	return hasAgent && column !== 'done' && column !== 'rejected'
}

export type RunErrorKind = 'not_connected' | 'no_provider' | 'message'

export interface RunErrorInfo {
	kind: RunErrorKind
	message: string
	stale: boolean
}

function fieldOf(error: unknown): {
	status?: number
	code?: string
	message?: string
} {
	return (error ?? {}) as { status?: number; code?: string; message?: string }
}

export function describeRunError(error: unknown): RunErrorInfo {
	const { status, code, message } = fieldOf(error)
	const info = (text: string, stale = false): RunErrorInfo => ({
		kind: 'message',
		message: text,
		stale,
	})
	if (status === 409) {
		if (code === 'not_connected') {
			return {
				kind: 'not_connected',
				message: message ?? "Un connecteur de l'agent n'est pas connecté.",
				stale: false,
			}
		}
		if (code === 'no_provider') {
			return {
				kind: 'no_provider',
				message:
					message ??
					'Aucun fournisseur LLM connecté. Connectez-en un pour lancer cet agent.',
				stale: false,
			}
		}
		if (code === 'execution_already_active') {
			return info('Une exécution est déjà en cours pour cette tâche.')
		}
		return info('Cette tâche a changé entre-temps. Réessaie.', true)
	}
	if (status === 422) {
		switch (code) {
			case 'card_without_agent':
				return info("Cette tâche n'a pas d'agent assigné.")
			case 'agent_archived':
				return info("L'agent assigné est archivé : assigne-en un autre.")
			case 'agent_without_connector':
				return info(
					"L'agent n'a aucun connecteur autorisé : accorde-lui en un.",
				)
			default:
				return info("Cette tâche ne peut pas être exécutée par l'agent.")
		}
	}
	if (status === 403) return info("Tu n'as pas le droit d'exécuter cet agent.")
	return info("Le lancement de l'agent a échoué. Réessaie.")
}
